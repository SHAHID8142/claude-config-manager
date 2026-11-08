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
    default_helpers_dir, generate_helper_script_for, install_helper_script,
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
/// Optional Claude Code model-role overrides (T45, PRD F05).
///
/// Every field defaults to `None`, which means **no override**: the existing
/// managed env key is left exactly as it is. Claude Code has documented
/// defaults for each role, so the app never invents a mapping — it only
/// writes what the user explicitly typed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RoleOverrides {
    pub haiku: Option<String>,
    pub sonnet: Option<String>,
    pub opus: Option<String>,
}

/// Trim, treat blank as "no override", and validate the exact value written.
fn normalize_role_model(value: Option<&str>) -> Result<Option<String>, ApplyError> {
    let value = value.map(str::trim).filter(|v| !v.is_empty());
    if let Some(m) = value {
        crate::config_merge::validate_model_id(m).map_err(|e| {
            ApplyError::InvalidInput(format!("role model override is invalid: {e}"))
        })?;
    }
    Ok(value.map(|v| v.to_string()))
}

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
    /// Directory holding the apply lock. Sits beside backups so both live in
    /// the app's support folder rather than next to the user's settings file.
    pub fn lock_dir(&self) -> PathBuf {
        self.backups_dir
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| self.backups_dir.clone())
    }

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
        self.apply_with_roles(
            provider_id,
            model,
            base_url,
            api_key,
            &RoleOverrides::default(),
        )
    }

    /// Same pipeline as [`Self::apply`], plus explicit model-role overrides
    /// (T45). An empty/absent role leaves the corresponding managed env key
    /// untouched — the "default-no-override" behaviour PRD F05 requires.
    pub fn apply_with_roles(
        &self,
        provider_id: &str,
        model: Option<&str>,
        base_url: Option<&str>,
        api_key: Option<&str>,
        roles: &RoleOverrides,
    ) -> Result<ApplyOutcome, ApplyError> {
        self.apply_with_window(
            provider_id,
            model,
            base_url,
            api_key,
            roles,
            None,
            false,
            None,
        )
    }

    /// [`Self::apply_with_roles`] plus the declared context window
    /// (`CLAUDE_CODE_MAX_CONTEXT_TOKENS`, V2-12b). `None` leaves the managed
    /// key untouched; `reset_window` removes it so Claude Code falls back to
    /// its own assumption.
    #[allow(clippy::too_many_arguments)]
    pub fn apply_with_window(
        &self,
        provider_id: &str,
        model: Option<&str>,
        base_url: Option<&str>,
        api_key: Option<&str>,
        roles: &RoleOverrides,
        context_window: Option<u32>,
        reset_window: bool,
        credential_account: Option<&str>,
    ) -> Result<ApplyOutcome, ApplyError> {
        // --- Mutual exclusion (G4): one apply at a time, process-wide ---
        // Acquired before anything is read so two windows cannot interleave a
        // read-merge-write cycle. Released automatically when this returns.
        let _lock = crate::apply_lock::ApplyLock::acquire(&self.lock_dir())
            .map_err(|e| ApplyError::Io(e.to_string()))?;

        // --- Input validation (nothing touched yet) ---
        crate::credential_handoff::validate_provider_id(provider_id)
            .map_err(|_| ApplyError::InvalidInput("invalid provider id".into()))?;
        // T43: normalize once, then validate the EXACT value we will write.
        // `validate_model_id` judges the trimmed form, so validating an
        // untrimmed input would let `"  claude-sonnet-4-5  "` through and
        // persist padded whitespace that Claude Code cannot resolve.
        let model: Option<&str> = model.map(str::trim).filter(|m| !m.is_empty());
        if let Some(m) = model {
            crate::config_merge::validate_model_id(m)
                .map_err(|e| ApplyError::InvalidInput(e.to_string()))?;
        }
        // T45: role overrides are validated with the same rules as `model`.
        let roles = RoleOverrides {
            haiku: normalize_role_model(roles.haiku.as_deref())?,
            sonnet: normalize_role_model(roles.sonnet.as_deref())?,
            opus: normalize_role_model(roles.opus.as_deref())?,
        };
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
        // V2-21: with several credentials per provider the helper must read the
        // one the owner selected, not a provider-wide default file.
        let account = credential_account.unwrap_or(provider_id);
        let stored = creds
            .has_credential(account)
            .map_err(|e| ApplyError::Io(e.to_string()))?;
        if !stored {
            return Err(ApplyError::NoCredential);
        }
        let script = generate_helper_script_for(provider_id, account)
            .map_err(|e| ApplyError::Io(e.to_string()))?;
        let helper_path = install_helper_script(&self.helpers_dir, provider_id, &script)
            .map_err(|e| ApplyError::Backup(e.to_string()))?;
        applied_keys.push("apiKeyHelper".to_string());

        // --- Merge (T28) ---
        let desired = DesiredSettings {
            model: model.map(|m| m.to_string()),
            env_base_url: base_url.map(|b| b.to_string()),
            // M1: a previously applied provider endpoint must not survive a
            // switch to one that has none (native Anthropic) — the key would
            // otherwise be sent to the previous provider.
            clear_env_base_url: base_url.is_none(),
            api_key_helper: Some(helper_path.to_string_lossy().to_string()),
            // The credential now lives in the helper only; plaintext env keys
            // would outrank it and silently make the stored key dead weight.
            clear_env_credentials: true,
            // Same reasoning for the model: ANTHROPIC_MODEL wins over
            // settings.model, so a stale `auto` would silently override the pick.
            clear_env_model: model.is_some(),
            env_default_haiku: roles.haiku,
            env_default_sonnet: roles.sonnet,
            env_default_opus: roles.opus,
            env_max_context_tokens: context_window.map(|n| n.to_string()),
            clear_env_max_context_tokens: reset_window,
            ..Default::default()
        };

        self.write_desired(desired, applied_keys, &secret_refs)
    }

    /// V2-31: point Claude Code at the loopback failover proxy.
    ///
    /// The helper script returns the proxy's bearer token, the base URL is the
    /// loopback listener, and every provider credential stays in the owner-only
    /// store: the proxy is the only component that reads them.
    pub fn apply_routed(
        &self,
        port: u16,
        token: &str,
        model: Option<&str>,
    ) -> Result<ApplyOutcome, ApplyError> {
        // Same one-apply-at-a-time guarantee as every other entry point.
        let _lock = crate::apply_lock::ApplyLock::acquire(&self.lock_dir())
            .map_err(|e| ApplyError::Io(e.to_string()))?;
        let model: Option<&str> = model.map(str::trim).filter(|m| !m.is_empty());
        if let Some(m) = model {
            crate::config_merge::validate_model_id(m)
                .map_err(|e| ApplyError::InvalidInput(e.to_string()))?;
        }
        if token.len() < 16 {
            return Err(ApplyError::InvalidInput("proxy token is too short".into()));
        }
        // Store the token exactly like a provider credential so the same
        // permission checks and the same helper template apply.
        let store = crate::file_secret_store::FileSecretStore::for_home(&self.home);
        store
            .set_secret(
                "com.claude-config-manager.proxy",
                crate::secret_store::PROXY_ACCOUNT,
                token,
            )
            .map_err(|e| ApplyError::Io(e.to_string()))?;
        let script = generate_helper_script_for("proxy", crate::secret_store::PROXY_ACCOUNT)
            .map_err(|e| ApplyError::Io(e.to_string()))?;
        let helper_path = install_helper_script(&self.helpers_dir, "proxy", &script)
            .map_err(|e| ApplyError::Backup(e.to_string()))?;

        let desired = DesiredSettings {
            model: model.map(|m| m.to_string()),
            env_base_url: Some(format!("http://127.0.0.1:{port}")),
            clear_env_base_url: false,
            api_key_helper: Some(helper_path.to_string_lossy().to_string()),
            clear_env_credentials: true,
            clear_env_model: model.is_some(),
            ..Default::default()
        };
        let mut applied = vec!["apiKeyHelper".to_string(), "ANTHROPIC_BASE_URL".to_string()];
        if model.is_some() {
            applied.push("model".to_string());
        }
        let outcome = self.write_desired(desired, applied, &[token])?;
        Ok(outcome)
    }

    /// Shared commit stage: read, merge, back up, write atomically, verify.
    fn write_desired(
        &self,
        desired: DesiredSettings,
        applied_keys: Vec<String>,
        secret_refs: &[&str],
    ) -> Result<ApplyOutcome, ApplyError> {
        // Callers already hold the apply lock (see `apply_with_window`).
        let redact = |e: ApplyError| -> ApplyError {
            match e {
                ApplyError::Io(m) => ApplyError::Io(redact_secrets(&m, secret_refs)),
                other => other,
            }
        };
        let mut applied_keys = applied_keys;
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
            helper_path: desired.api_key_helper.clone().unwrap_or_default(),
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
    fn routed_mode_points_claude_code_at_the_loopback_proxy() {
        let (home, store) = fixture("routed");
        let svc = service(&home, store);
        let token = "0123456789abcdef0123456789abcdef";
        let out = svc
            .apply_routed(51234, token, Some("nvidia/nemotron-3-ultra"))
            .expect("routed apply");

        let written: Value =
            serde_json::from_str(&fs::read_to_string(home.join(".claude/settings.json")).unwrap())
                .expect("json");
        // Loopback only, and no plaintext credential anywhere in settings.
        assert_eq!(
            written["env"]["ANTHROPIC_BASE_URL"],
            "http://127.0.0.1:51234"
        );
        assert!(written["env"].get("ANTHROPIC_AUTH_TOKEN").is_none());
        assert!(written["env"].get("ANTHROPIC_API_KEY").is_none());
        assert_eq!(written["model"], "nvidia/nemotron-3-ultra");
        assert!(out.helper_path.ends_with("proxy.sh"));

        // The token lives in a 0600 file and the helper returns it verbatim.
        let secret = home.join(".claude-config-manager/secrets/proxy.secret");
        let mode = fs::metadata(&secret)
            .expect("token file")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600, "proxy token must be owner-only");
        let script_path = home.join(&out.helper_path);
        let output = std::process::Command::new("sh")
            .arg(&script_path)
            .env("HOME", &home)
            .output()
            .expect("run helper");
        assert!(output.status.success(), "helper failed: {:?}", output);
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), token);
    }

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

    fn read_env(home: &Path) -> Value {
        let text = fs::read_to_string(home.join(".claude/settings.json")).expect("read settings");
        let doc: Value = serde_json::from_str(&text).expect("valid json");
        doc.get("env").cloned().unwrap_or(Value::Null)
    }

    #[test]
    fn role_overrides_are_written_when_the_user_sets_them() {
        let (home, store) = fixture("roles");
        let svc = service(&home, store);
        let roles = RoleOverrides {
            haiku: Some("  fast-model  ".into()),
            sonnet: Some("main-model".into()),
            opus: None,
        };
        svc.apply_with_roles("deepseek", None, None, Some(FAKE_KEY), &roles)
            .expect("apply with roles");

        let env = read_env(&home);
        assert_eq!(env["ANTHROPIC_DEFAULT_HAIKU_MODEL"], "fast-model");
        assert_eq!(env["ANTHROPIC_DEFAULT_SONNET_MODEL"], "main-model");
        assert_eq!(
            env.get("ANTHROPIC_DEFAULT_OPUS_MODEL"),
            None,
            "absent role must not be invented"
        );
        // Unrelated pre-existing env survives the merge.
        assert_eq!(env["CLAUDE_CODE_X"], "1");
    }

    #[test]
    fn absent_or_blank_role_overrides_leave_existing_values_untouched() {
        let (home, store) = fixture("roles-noop");
        fs::write(
            home.join(".claude/settings.json"),
            r#"{"model":"old-model","env":{"ANTHROPIC_DEFAULT_OPUS_MODEL":"preexisting-opus"}}"#,
        )
        .expect("seed");
        let svc = service(&home, store);

        // All blank/absent → default-no-override: nothing about roles changes.
        let blank = RoleOverrides {
            haiku: Some("   ".into()),
            sonnet: None,
            opus: Some("".into()),
        };
        svc.apply_with_roles("deepseek", None, None, Some(FAKE_KEY), &blank)
            .expect("apply");

        let env = read_env(&home);
        assert_eq!(env["ANTHROPIC_DEFAULT_OPUS_MODEL"], "preexisting-opus");
        assert_eq!(env.get("ANTHROPIC_DEFAULT_HAIKU_MODEL"), None);
        assert_eq!(env.get("ANTHROPIC_DEFAULT_SONNET_MODEL"), None);
    }

    /// T46: Claude Code documents ANTHROPIC_DEFAULT_*_MODEL as *pinning an
    /// alias*; there is no documented failover chain. The app must therefore
    /// never write a second competing model key (env.ANTHROPIC_MODEL) or a
    /// fallback list — one explicit model, or none at all.
    #[test]
    fn apply_never_writes_a_second_model_key_or_a_failover_chain() {
        let (home, store) = fixture("no-failover");
        let svc = service(&home, store);
        svc.apply("deepseek", Some("main-model"), None, Some(FAKE_KEY))
            .expect("apply");

        let text = fs::read_to_string(home.join(".claude/settings.json")).expect("read");
        let doc: Value = serde_json::from_str(&text).expect("json");
        assert_eq!(doc["model"], "main-model");
        assert_eq!(
            doc["env"].get("ANTHROPIC_MODEL"),
            None,
            "app must not set a second model source that would shadow/duplicate `model`"
        );
        // No array/chain shape anywhere: every model-ish value is a string.
        assert!(doc["model"].is_string());
        for key in [
            "ANTHROPIC_DEFAULT_HAIKU_MODEL",
            "ANTHROPIC_DEFAULT_SONNET_MODEL",
            "ANTHROPIC_DEFAULT_OPUS_MODEL",
        ] {
            assert!(
                doc["env"].get(key).map(|v| v.is_string()).unwrap_or(true),
                "{key} must be a single id, never a chain"
            );
        }
    }

    /// T46: a rejected apply must not "fall back" to a model it likes better.
    /// The previously applied configuration stays exactly as it was.
    #[test]
    fn failed_apply_keeps_the_previous_model_instead_of_failing_over() {
        let (home, store) = fixture("no-fallback");
        let svc = service(&home, store);
        svc.apply_with_roles(
            "deepseek",
            Some("good-model"),
            None,
            Some(FAKE_KEY),
            &RoleOverrides {
                sonnet: Some("role-sonnet".into()),
                ..Default::default()
            },
        )
        .expect("first apply");
        let before = fs::read(home.join(".claude/settings.json")).expect("read");

        // Second apply is rejected (invalid model id) → nothing may change,
        // and certainly no substitute model may appear.
        let err = svc
            .apply("deepseek", Some("bad model"), None, Some(FAKE_KEY))
            .expect_err("invalid model rejected");
        assert!(matches!(err, ApplyError::InvalidInput(_)));

        let after = fs::read(home.join(".claude/settings.json")).expect("read");
        assert_eq!(before, after, "no fallback write allowed after a rejection");
        let doc: Value = serde_json::from_slice(&after).expect("json");
        assert_eq!(doc["model"], "good-model");
        assert_eq!(doc["env"]["ANTHROPIC_DEFAULT_SONNET_MODEL"], "role-sonnet");
    }

    /// G4: eight concurrent applies over the same settings file. Exactly one may
    /// hold the lock at a time; the rest must be refused with a clear message
    /// rather than corrupting the file.
    #[test]
    fn concurrent_applies_are_serialized_and_never_corrupt_the_file() {
        let (home, _store) = fixture("g4-concurrent");
        let home = Arc::new(home);
        let results: Vec<_> = (0..8)
            .map(|i| {
                let home = Arc::clone(&home);
                std::thread::spawn(move || {
                    // Separate service instances = separate app instances.
                    let svc = service(&home, Arc::new(MockSecretStore::new()));
                    svc.apply(
                        "deepseek",
                        Some(&format!("model-{i}")),
                        Some("https://api.deepseek.com/anthropic"),
                        Some(FAKE_KEY),
                    )
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|h| h.join().expect("thread"))
            .collect();

        let ok = results.iter().filter(|r| r.is_ok()).count();
        assert!(ok >= 1, "at least one apply must succeed: {results:?}");
        for r in &results {
            if let Err(e) = r {
                let msg = e.to_string();
                assert!(
                    msg.contains("already running"),
                    "a concurrent apply may only fail with the lock message, got: {msg}"
                );
            }
        }

        // Whatever won, the file is valid JSON with a model one of us wrote and
        // the user's unrelated env key intact.
        let doc: Value = serde_json::from_str(
            &fs::read_to_string(home.join(".claude/settings.json")).expect("read"),
        )
        .expect("settings must be valid JSON after concurrent applies");
        let model = doc["model"].as_str().expect("model").to_string();
        assert!(
            model.starts_with("model-"),
            "unexpected model after concurrency: {model}"
        );
        assert_eq!(doc["env"]["CLAUDE_CODE_X"], "1");
        // Lock must be released once every apply returned.
        assert!(
            !home.join("apply.lock").exists(),
            "the apply lock must not be left behind"
        );
    }

    /// M1 regression: switching from a compat provider back to native Anthropic
    /// must REMOVE the old base URL. Leaving it would send the Anthropic key to
    /// the previous provider — the worst failure mode in this app.
    #[test]
    fn switching_to_native_clears_the_previous_provider_endpoint() {
        let (home, store) = fixture("m1-switch");
        let svc = service(&home, store);
        svc.apply(
            "deepseek",
            Some("deepseek-chat"),
            Some("https://api.deepseek.com/anthropic"),
            Some(FAKE_KEY),
        )
        .expect("compat apply");
        assert_eq!(
            read_env(&home)["ANTHROPIC_BASE_URL"],
            "https://api.deepseek.com/anthropic"
        );

        // Now switch to native Anthropic: no base URL is requested at all.
        svc.apply("anthropic", Some("claude-sonnet-4-5"), None, Some(FAKE_KEY))
            .expect("native apply");

        let env = read_env(&home);
        assert_eq!(
            env.get("ANTHROPIC_BASE_URL"),
            None,
            "stale endpoint must be removed, not inherited"
        );
        // Unrelated env is preserved through the removal.
        assert_eq!(env["CLAUDE_CODE_X"], "1");
        let doc: Value = serde_json::from_str(
            &fs::read_to_string(home.join(".claude/settings.json")).expect("read"),
        )
        .expect("json");
        assert_eq!(doc["model"], "claude-sonnet-4-5");
    }

    /// The clear flag must not touch a settings file that has no endpoint.
    #[test]
    fn clearing_an_absent_endpoint_is_a_no_op() {
        let (home, store) = fixture("m1-noop");
        fs::write(
            home.join(".claude/settings.json"),
            r#"{"env":{"KEEP":"1"}}"#,
        )
        .expect("seed");
        let svc = service(&home, store);
        svc.apply("anthropic", None, None, Some(FAKE_KEY))
            .expect("apply");

        let env = read_env(&home);
        assert_eq!(env, serde_json::json!({"KEEP":"1"}));
    }

    #[test]
    fn invalid_role_override_is_rejected_before_any_write() {
        let (home, store) = fixture("roles-bad");
        let before = fs::read(home.join(".claude/settings.json")).expect("read");
        let svc = service(&home, store);
        let roles = RoleOverrides {
            haiku: None,
            sonnet: Some("bad model with spaces".into()),
            opus: None,
        };
        let err = svc
            .apply_with_roles("deepseek", None, None, Some(FAKE_KEY), &roles)
            .expect_err("invalid role must be rejected");
        assert!(matches!(err, ApplyError::InvalidInput(ref m) if m.contains("role model")));
        let after = fs::read(home.join(".claude/settings.json")).expect("read");
        assert_eq!(
            before, after,
            "settings must be byte-identical after a rejection"
        );
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
    fn model_input_is_normalized_before_write() {
        let (home, store) = fixture("trim");
        let settings = home.join(".claude/settings.json");
        let svc = service(&home, store);

        // Padded id: validated AND written in trimmed form (T43).
        svc.apply(
            "anthropic",
            Some("  claude-sonnet-4-5  "),
            None,
            Some(FAKE_KEY),
        )
        .expect("apply padded");
        let doc: Value =
            serde_json::from_str(&fs::read_to_string(&settings).expect("read")).expect("json");
        assert_eq!(
            doc["model"], "claude-sonnet-4-5",
            "padded input must not persist whitespace"
        );

        // Whitespace-only id means "no model selected": the existing managed
        // value is left untouched rather than replaced with junk.
        svc.apply("anthropic", Some("   "), None, Some(FAKE_KEY))
            .expect("apply blank");
        let doc: Value =
            serde_json::from_str(&fs::read_to_string(&settings).expect("read")).expect("json");
        assert_eq!(doc["model"], "claude-sonnet-4-5");

        // Invalid characters are still rejected before any write.
        let before = fs::read_to_string(&settings).expect("read");
        assert!(matches!(
            svc.apply("anthropic", Some("bad model\n"), None, Some(FAKE_KEY)),
            Err(ApplyError::InvalidInput(_))
        ));
        assert_eq!(fs::read_to_string(&settings).expect("read"), before);

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
