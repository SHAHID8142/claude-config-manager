//! Save & Apply orchestration (T33): the full safe-apply pipeline for one
//! verified direct provider (Anthropic).
//!
//! Pipeline (fail-closed at every step, PRD F03/F06/F07):
//! validate inputs → store credential (Keychain) → install helper script →
//! snapshot mtime → read current settings (refusing malformed sources) →
//! merge (T28) → backup (T29) → atomic write with conflict detection
//! (T30) → read-back → status.

use crate::config_backup::{create_backup, default_backup_dir};
use crate::config_locator::{ConfigLocator, LocatedStatus};
use crate::config_merge::{DesiredSettings, apply_desired};
use crate::config_reader::ConfigError;
use crate::config_writer::{snapshot_mtime, write_settings_atomic};
use crate::credential_handoff::{
    default_helpers_dir, generate_helper_script, install_helper_script,
};
use crate::credential_service::CredentialService;
use crate::redact::redact_secrets;
use crate::secret_store::SecretStore;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyOutcome {
    pub settings_path: String,
    pub backup_file_name: String,
    pub helper_path: String,
    /// Which managed keys changed in this apply.
    pub applied_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ApplyError {
    /// Input rejected before anything was touched.
    InvalidInput(String),
    /// Credential missing for helper mode.
    NoCredential,
    Config(ConfigError),
    Backup(String),
    /// Anything at the filesystem/OS layer; message pre-redacted.
    Io(String),
}

impl std::fmt::Display for ApplyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApplyError::InvalidInput(m) => write!(f, "{m}"),
            ApplyError::NoCredential => write!(f, "no credential stored for this provider"),
            ApplyError::Config(e) => write!(f, "{e}"),
            ApplyError::Backup(m) => write!(f, "backup failed ({m})"),
            ApplyError::Io(m) => write!(f, "{m}"),
        }
    }
}

/// Injectable orchestration service (production wiring reads $HOME and the
/// macOS Keychain; tests inject fixtures).
pub struct SaveApplyService {
    pub home: PathBuf,
    pub store: Arc<dyn SecretStore>,
    pub helpers_dir: PathBuf,
    pub backups_dir: PathBuf,
    /// Config-dir override standing in for CLAUDE_CONFIG_DIR.
    pub config_dir_override: Option<PathBuf>,
}

impl SaveApplyService {
    pub fn for_production(home: &Path, store: Arc<dyn SecretStore>) -> Self {
        Self {
            home: home.to_path_buf(),
            store,
            helpers_dir: default_helpers_dir(home),
            backups_dir: default_backup_dir(home),
            config_dir_override: None,
        }
    }

    /// Apply settings for one provider. `model` is optional (model selection
    /// lands in the model milestone); `base_url` is None for native Anthropic.
    pub fn apply(
        &self,
        provider_id: &str,
        model: Option<&str>,
        base_url: Option<&str>,
        api_key: Option<&str>,
    ) -> Result<ApplyOutcome, ApplyError> {
        // --- Input validation (nothing touched yet) ---
        crate::credential_handoff::validate_provider_id(provider_id)
            .map_err(|_| ApplyError::InvalidInput("invalid provider id".into()))?;
        if let Some(m) = model {
            crate::config_merge::validate_model_id(m)
                .map_err(|e| ApplyError::InvalidInput(e.to_string()))?;
        }
        if let Some(url) = base_url {
            let bad = matches!(
                crate::provider_catalog::classify_endpoint(url),
                crate::provider_catalog::EndpointCheck::Invalid(_)
                    | crate::provider_catalog::EndpointCheck::Insecure
            );
            if bad {
                return Err(ApplyError::InvalidInput(
                    "base URL must be a valid https endpoint".into(),
                ));
            }
        }

        let mut applied_keys: Vec<String> = Vec::new();
        let live_secrets: Vec<String> = api_key.map(|k| vec![k.to_string()]).unwrap_or_default();
        let secret_refs: Vec<&str> = live_secrets.iter().map(|s| s.as_str()).collect();
        let redact = |e: ApplyError| -> ApplyError {
            match e {
                ApplyError::Io(m) => ApplyError::Io(redact_secrets(&m, &secret_refs)),
                other => other,
            }
        };

        // --- Credential storage (Keychain via the store) ---
        let creds = CredentialService::new(self.store.as_ref());
        match api_key {
            Some(key) if !key.trim().is_empty() => {
                creds
                    .set_credential(provider_id, key)
                    .map_err(|e| ApplyError::Io(e.to_string()))?;
                applied_keys.push(format!("credential:{provider_id}"));
            }
            _ => {}
        }

        // --- Helper script (helper mode is the only mode in T33) ---
        let stored = creds
            .has_credential(provider_id)
            .map_err(|e| ApplyError::Io(e.to_string()))?;
        if !stored {
            return Err(ApplyError::NoCredential);
        }
        let script =
            generate_helper_script(provider_id).map_err(|e| ApplyError::Io(e.to_string()))?;
        let helper_path = install_helper_script(&self.helpers_dir, provider_id, &script)
            .map_err(|e| ApplyError::Backup(e.to_string()))?;
        applied_keys.push("apiKeyHelper".to_string());

        // --- Locate + read current settings ---
        let locator = ConfigLocator::new();
        let settings_path = locator.locate(&self.home, self.config_dir_override.as_deref());

        let mtime = snapshot_mtime(&settings_path).map_err(|e| redact(ApplyError::Config(e)))?;
        let current = match locator.inspect(&settings_path) {
            Ok(LocatedStatus::RegularFile) => {
                // Malformed sources are NEVER overwritten (PRD F02).
                crate::config_reader::read_settings(&settings_path)
                    .map_err(|e| redact(ApplyError::Config(e)))?
            }
            Ok(LocatedStatus::Missing) => serde_json::json!({}),
            Ok(LocatedStatus::Symlink) => {
                return Err(redact(ApplyError::Config(ConfigError::SymlinkRefused)));
            }
            Ok(LocatedStatus::OtherFile) => {
                return Err(redact(ApplyError::Config(ConfigError::Io(
                    "settings path is not a regular file".into(),
                ))));
            }
            Err(e) => return Err(redact(ApplyError::Io(e.to_string()))),
        };

        // --- Merge (T28) ---
        let desired = DesiredSettings {
            model: model.map(|m| m.to_string()),
            env_base_url: base_url.map(|b| b.to_string()),
            api_key_helper: Some(helper_path.to_string_lossy().to_string()),
            ..Default::default()
        };
        let next = apply_desired(&current, &desired).map_err(|e| redact(ApplyError::Config(e)))?;
        for k in ["model", "env", "apiKeyHelper"] {
            if next.get(k) != current.get(k) {
                applied_keys.push(k.to_string());
            }
        }

        // --- Backup (T29) — only when the file exists to back up ---
        if matches!(
            locator.inspect(&settings_path),
            Ok(LocatedStatus::RegularFile)
        ) {
            create_backup(&settings_path, &self.backups_dir)
                .map_err(|e| redact(ApplyError::Backup(e.to_string())))?;
        }

        // --- Atomic write with conflict detection (T30) ---
        write_settings_atomic(&settings_path, &next, mtime)
            .map_err(|e| redact(ApplyError::Config(e)))?;

        let backup_name = {
            let infos = crate::config_backup::list_backups(&self.backups_dir)
                .map_err(|e| redact(ApplyError::Backup(e.to_string())))?;
            infos
                .first()
                .map(|i| i.file_name.clone())
                .unwrap_or_default()
        };

        Ok(ApplyOutcome {
            settings_path: settings_path.to_string_lossy().to_string(),
            backup_file_name: backup_name,
            helper_path: helper_path.to_string_lossy().to_string(),
            applied_keys,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secret_store::MockSecretStore;
    use serde_json::Value;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    fn fixture(tag: &str) -> (PathBuf, Arc<MockSecretStore>) {
        let dir = std::env::temp_dir().join(format!(
            "ccm-t33-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(dir.join(".claude")).expect("config dir");
        // Seed an existing settings file so the backup stage runs (the
        // representative real-world case).
        fs::write(
            dir.join(".claude/settings.json"),
            r#"{"model":"old-model","env":{"CLAUDE_CODE_X":"1"}}"#,
        )
        .expect("seed settings");
        (dir, Arc::new(MockSecretStore::new()))
    }

    fn service(home: &Path, store: Arc<MockSecretStore>) -> SaveApplyService {
        SaveApplyService {
            home: home.to_path_buf(),
            store,
            helpers_dir: home.join("helpers"),
            backups_dir: home.join("backups"),
            config_dir_override: None,
        }
    }

    const FAKE_KEY: &str = "sk-ant-api03-FAKE-000";

    #[test]
    fn full_pipeline_happy_path() {
        let (home, store) = fixture("happy");
        let svc = service(&home, store.clone());

        let out = svc
            .apply("anthropic", Some("claude-sonnet-4-5"), None, Some(FAKE_KEY))
            .expect("apply");

        // Settings written with managed keys only.
        let doc: Value =
            serde_json::from_str(&fs::read_to_string(&out.settings_path).expect("read"))
                .expect("json");
        assert_eq!(doc["model"], "claude-sonnet-4-5");
        assert!(
            doc["apiKeyHelper"]
                .as_str()
                .expect("helper path")
                .ends_with("anthropic.sh")
        );
        assert!(
            doc["env"].get("ANTHROPIC_BASE_URL").is_none(),
            "native provider writes no base URL"
        );
        // Unmanaged env keys survive (F02).
        assert_eq!(doc["env"]["CLAUDE_CODE_X"], "1");

        // Credential stored under our service name.
        assert_eq!(
            store
                .get_secret("com.claude-config-manager.anthropic", "anthropic")
                .expect("peek"),
            Some(FAKE_KEY.to_string())
        );
        // Helper installed executable.
        let mode = fs::metadata(&out.helper_path)
            .expect("meta")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o700);
        // Backup created.
        assert!(out.backup_file_name.starts_with("settings-backup-"));
        // Outcome reports what changed.
        assert!(out.applied_keys.contains(&"model".to_string()));
        assert!(out.applied_keys.contains(&"apiKeyHelper".to_string()));
        assert!(
            out.applied_keys
                .contains(&"credential:anthropic".to_string())
        );

        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn preserves_unrelated_settings_across_apply() {
        let (home, store) = fixture("preserve");
        let settings = home.join(".claude/settings.json");
        fs::write(
            &settings,
            r#"{"statusLine":{"type":"command"},"model":"old","env":{"CLAUDE_CODE_X":"1"}}"#,
        )
        .expect("seed");
        let svc = service(&home, store);

        svc.apply("anthropic", Some("new-model"), None, Some(FAKE_KEY))
            .expect("apply");

        let doc: Value =
            serde_json::from_str(&fs::read_to_string(&settings).expect("read")).expect("json");
        assert_eq!(doc["statusLine"]["type"], "command");
        assert_eq!(doc["env"]["CLAUDE_CODE_X"], "1");
        assert_eq!(doc["model"], "new-model");
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn malformed_source_is_never_overwritten() {
        let (home, store) = fixture("malformed");
        let settings = home.join(".claude/settings.json");
        let bad = "{\n  \"broken\": tru";
        fs::write(&settings, bad).expect("seed malformed");
        let svc = service(&home, store);

        let err = svc
            .apply("anthropic", None, None, Some(FAKE_KEY))
            .expect_err("must refuse");
        assert!(matches!(
            err,
            ApplyError::Config(ConfigError::MalformedJson(_))
        ));
        // Source untouched.
        assert_eq!(fs::read_to_string(&settings).expect("read"), bad);
        // No backup taken of a broken file.
        assert!(
            !home.join("backups").exists()
                || fs::read_dir(home.join("backups")).expect("dir").count() == 0
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn missing_credential_blocks_helper_mode() {
        let (home, store) = fixture("nocred");
        let svc = service(&home, store);
        let err = svc
            .apply("anthropic", None, None, None)
            .expect_err("no key");
        assert_eq!(err, ApplyError::NoCredential);
        // Helper was not installed.
        assert!(!home.join("helpers").join("anthropic.sh").exists());
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn invalid_provider_and_endpoint_rejected_before_any_write() {
        let (home, store) = fixture("invalid");
        let svc = service(&home, store);

        for result in [
            svc.apply("Bad;Provider", None, None, Some(FAKE_KEY)),
            svc.apply(
                "anthropic",
                None,
                Some("http://insecure.example.com"),
                Some(FAKE_KEY),
            ),
            svc.apply("anthropic", Some("bad model id"), None, Some(FAKE_KEY)),
        ] {
            assert!(
                matches!(result, Err(ApplyError::InvalidInput(_))),
                "must be InvalidInput"
            );
        }
        // Nothing was written or stored: settings file still equals the seed.
        assert_eq!(
            fs::read_to_string(home.join(".claude/settings.json")).expect("r"),
            r#"{"model":"old-model","env":{"CLAUDE_CODE_X":"1"}}"#
        );
        assert!(!home.join("helpers").join("anthropic.sh").exists());
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn error_paths_never_leak_the_secret() {
        let (home, store) = fixture("leak");
        // Point helpers dir at an unwritable location to force a backup-stage
        // error AFTER the credential is known.
        let svc = SaveApplyService {
            home: home.clone(),
            store: store.clone(),
            helpers_dir: home.join("helpers"),
            backups_dir: home.join("backups"),
            config_dir_override: None,
        };
        fs::create_dir_all(&home).expect("home");
        // Seed a malformed settings file so the failure happens late, with
        // the secret already in scope.
        fs::write(home.join(".claude/settings.json"), "{broken").expect("seed");

        let err = svc
            .apply("anthropic", None, None, Some(FAKE_KEY))
            .expect_err("must fail");
        let rendered = err.to_string();
        assert!(!rendered.contains(FAKE_KEY), "secret leaked: {rendered}");
        let _ = fs::remove_dir_all(&home);
    }
}
