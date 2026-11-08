//! File-backed credential store (replaces the Keychain path).
//!
//! Why: macOS refused `SecItemAdd` for the shipped bundle and no amount of
//! signing/re-signing reproduced or fixed it, so the Keychain made the core
//! feature unusable. A local file with owner-only permissions is the model every
//! other CLI tool already uses on macOS (`~/.ssh/id_ed25519`, `~/.aws/credentials`,
//! `~/.config/gcloud`), needs no entitlement, no prompt, and no code signature.
//!
//! Honest scope of the protection (also in docs/USER_GUIDE.md):
//! - Any process running **as the same user** can read the file. The Keychain's
//!   extra guarantee was per-item ACL prompts, and that is exactly what was
//!   failing. There is no OS API on macOS to encrypt a secret so that the same
//!   user cannot decrypt it — a key stored next to the ciphertext would be
//!   theatre, so this store does not pretend to encrypt.
//! - Group/other access is impossible: the directory is 0700 and the file 0600,
//!   both created with the mode up front (never chmod-after-write), and a file
//!   whose permissions were loosened is refused on read instead of trusted.
//! - The file lives outside `~/.claude`, so a dotfiles repo that syncs
//!   `~/.claude` never picks up the credential.
//!
//! Secret handling: values are written atomically (temp + `fsync` + rename), never
//! logged, never placed in `settings.json`, and never interpolated into the helper
//! script — the script only contains the derived path.

use crate::secret_store::{SecretError, SecretStore};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

/// Refuse absurd values before they reach the disk.
const MAX_SECRET_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone)]
pub struct FileSecretStore {
    root: PathBuf,
}

impl FileSecretStore {
    /// `<home>/Library/Application Support/ClaudeConfigManager/secrets`
    pub fn for_home(home: &Path) -> Self {
        Self {
            root: home
                .join("Library")
                .join("Application Support")
                .join("ClaudeConfigManager")
                .join("secrets"),
        }
    }

    #[cfg(test)]
    pub fn at(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    /// The file a service/account pair is stored in.
    ///
    /// The name is derived from the *account* (the provider id), and the service
    /// must be exactly ours for that account. Two different identifiers can
    /// therefore never collide on one file, and the helper script — which builds
    /// `<provider>.secret` from the validated provider id — addresses exactly the
    /// file this store writes.
    fn path_for(&self, service: &str, account: &str) -> Result<PathBuf, SecretError> {
        validate_provider_component(account)?;
        let expected = format!("com.claude-config-manager.{account}");
        if service != expected {
            return Err(SecretError::InvalidIdentifier);
        }
        Ok(self.root.join(format!("{account}.secret")))
    }

    fn ensure_root(&self) -> Result<(), SecretError> {
        if let Ok(meta) = std::fs::symlink_metadata(&self.root)
            && meta.file_type().is_symlink()
        {
            return Err(SecretError::Platform(
                "credential directory is a symlink; refusing".into(),
            ));
        }
        std::fs::create_dir_all(&self.root).map_err(|e| SecretError::Platform(e.to_string()))?;
        // 0700 for the directory, set at creation and re-asserted (create_dir_all
        // does not tighten an existing directory).
        std::fs::set_permissions(&self.root, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| SecretError::Platform(e.to_string()))?;
        Ok(())
    }
}

/// Provider ids are `[a-z0-9-]`, non-empty, not starting with `-`: the same rule
/// the helper script and the catalog use, so a filename can never contain a path
/// separator, a leading dot, or a shell-active character.
fn validate_provider_component(value: &str) -> Result<(), SecretError> {
    let starts_safely = value
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    let ok = !value.is_empty()
        && value.len() <= 64
        && starts_safely
        && value
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if ok {
        Ok(())
    } else {
        Err(SecretError::InvalidIdentifier)
    }
}

impl SecretStore for FileSecretStore {
    fn set_secret(&self, service: &str, account: &str, secret: &str) -> Result<(), SecretError> {
        let path = self.path_for(service, account)?;
        if secret.is_empty() {
            return Err(SecretError::Platform(
                "refusing to store an empty credential".into(),
            ));
        }
        if secret.len() > MAX_SECRET_BYTES {
            return Err(SecretError::Platform(format!(
                "credential is too large ({} bytes, limit {MAX_SECRET_BYTES})",
                secret.len()
            )));
        }
        if secret.chars().any(|c| c.is_control()) {
            // A newline inside the value would be indistinguishable from the
            // trailing newline we strip on read.
            return Err(SecretError::Platform(
                "credential contains control characters".into(),
            ));
        }
        self.ensure_root()?;

        let tmp = path.with_extension("secret.tmp");
        // Remove a stale temp so `create_new` cannot fail on our own leftovers.
        let _ = std::fs::remove_file(&tmp);

        {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600) // created owner-only, not chmod after the fact
                .open(&tmp)
                .map_err(|e| SecretError::Platform(e.to_string()))?;
            file.write_all(secret.as_bytes())
                .map_err(|e| SecretError::Platform(e.to_string()))?;
            file.sync_all()
                .map_err(|e| SecretError::Platform(e.to_string()))?;
        }
        // Refuse to replace a symlink placed where our file belongs.
        if let Ok(meta) = std::fs::symlink_metadata(&path)
            && meta.file_type().is_symlink()
        {
            let _ = std::fs::remove_file(&tmp);
            return Err(SecretError::Platform(
                "credential file is a symlink; refusing to write through it".into(),
            ));
        }
        std::fs::rename(&tmp, &path).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            SecretError::Platform(e.to_string())
        })?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| SecretError::Platform(e.to_string()))?;
        Ok(())
    }

    fn get_secret(&self, service: &str, account: &str) -> Result<Option<String>, SecretError> {
        let path = self.path_for(service, account)?;
        let meta = match std::fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(SecretError::Platform(e.to_string())),
        };
        if meta.file_type().is_symlink() {
            return Err(SecretError::Platform(
                "credential file is a symlink; refusing to read through it".into(),
            ));
        }
        if meta.permissions().mode() & 0o077 != 0 {
            // Someone loosened the file: a group/world-readable credential is a
            // finding, not something to silently keep using.
            return Err(SecretError::Platform(format!(
                "credential file permissions are too open ({:o}); expected 600",
                meta.permissions().mode() & 0o777
            )));
        }
        if meta.len() as usize > MAX_SECRET_BYTES + 1 {
            return Err(SecretError::Platform(
                "credential file is unexpectedly large".into(),
            ));
        }
        let raw = std::fs::read(&path).map_err(|e| SecretError::Platform(e.to_string()))?;
        let text = String::from_utf8(raw)
            .map_err(|_| SecretError::Platform("credential file is not valid UTF-8".into()))?;
        Ok(Some(text.trim_end_matches(['\n', '\r']).to_string()))
    }

    fn delete_secret(&self, service: &str, account: &str) -> Result<(), SecretError> {
        let path = self.path_for(service, account)?;
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(SecretError::Platform(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    fn temp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ccm-filestore-{tag}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("dir");
        dir
    }

    const SVC: &str = "com.claude-config-manager.deepseek";
    const ACCT: &str = "deepseek";

    #[test]
    fn round_trips_a_secret_and_creates_owner_only_files() {
        let root = temp_root("rt");
        let store = FileSecretStore::at(&root);
        store
            .set_secret(SVC, ACCT, "sk-ant-FAKE-roundtrip")
            .expect("set");
        assert_eq!(
            store.get_secret(SVC, ACCT).expect("get"),
            Some("sk-ant-FAKE-roundtrip".into())
        );
        let file = root.join("deepseek.secret");
        let mode = std::fs::metadata(&file).expect("meta").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "credential file must be 0600, got {mode:o}");
        let dir_mode = std::fs::metadata(&root).expect("meta").permissions().mode() & 0o777;
        assert_eq!(
            dir_mode, 0o700,
            "credential dir must be 0700, got {dir_mode:o}"
        );
        store.delete_secret(SVC, ACCT).expect("delete");
        assert_eq!(store.get_secret(SVC, ACCT).expect("get"), None);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn replacing_keeps_exactly_one_file_and_no_temp_residue() {
        let root = temp_root("replace");
        let store = FileSecretStore::at(&root);
        store.set_secret(SVC, ACCT, "first-value").expect("set");
        store
            .set_secret(SVC, ACCT, "second-value")
            .expect("replace");
        assert_eq!(
            store.get_secret(SVC, ACCT).expect("get"),
            Some("second-value".into())
        );
        let names: Vec<String> = std::fs::read_dir(&root)
            .expect("dir")
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(
            names,
            vec!["deepseek.secret".to_string()],
            "residue: {names:?}"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_loosened_file_is_refused_rather_than_trusted() {
        let root = temp_root("loose");
        let store = FileSecretStore::at(&root);
        store.set_secret(SVC, ACCT, "value").expect("set");
        std::fs::set_permissions(
            root.join("deepseek.secret"),
            std::fs::Permissions::from_mode(0o644),
        )
        .expect("chmod");
        let err = store.get_secret(SVC, ACCT).expect_err("must refuse");
        assert!(matches!(err, SecretError::Platform(ref m) if m.contains("permissions")));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_symlink_is_never_followed_in_either_direction() {
        let root = temp_root("symlink");
        let target = root.join("outside.txt");
        std::fs::write(&target, "attacker-controlled").expect("write");
        std::fs::create_dir_all(&root).expect("dir");
        std::os::unix::fs::symlink(&target, root.join("deepseek.secret")).expect("symlink");

        let store = FileSecretStore::at(&root);
        assert!(
            store.get_secret(SVC, ACCT).is_err(),
            "must not read through a symlink"
        );
        assert!(
            store.set_secret(SVC, ACCT, "new-value").is_err(),
            "must not write through a symlink"
        );
        assert_eq!(
            std::fs::read_to_string(&target).expect("read target"),
            "attacker-controlled"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn hostile_identifiers_cannot_escape_the_directory() {
        let root = temp_root("traversal");
        let store = FileSecretStore::at(&root);
        for (svc, acct) in [
            ("com.claude-config-manager../evil", "../evil"),
            ("com.claude-config-manager..hidden", "..hidden"),
            ("com.claude-config-manager.deepseek", "acct/../../evil"),
            ("com.claude-config-manager.-flag", "-flag"),
            ("com.claude-config-manager.", ""),
            (
                &format!("com.claude-config-manager.{}", "a".repeat(200)),
                &"a".repeat(200),
            ),
            // A service that is not ours for that account must not map to a file.
            ("com.someone-else.deepseek", "deepseek"),
        ] {
            assert!(
                store.path_for(svc, acct).is_err(),
                "identifier {svc:?}/{acct:?} must be refused"
            );
        }
        // ...while the legitimate pair resolves to exactly the file the helper reads.
        assert_eq!(
            store.path_for(SVC, ACCT).expect("valid pair"),
            root.join("deepseek.secret")
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn empty_or_control_bearing_secrets_are_refused() {
        let root = temp_root("empty");
        let store = FileSecretStore::at(&root);
        assert!(store.set_secret(SVC, ACCT, "").is_err());
        assert!(store.set_secret(SVC, ACCT, "line1\nline2").is_err());
        assert!(
            store
                .set_secret(SVC, ACCT, &"x".repeat(MAX_SECRET_BYTES + 1))
                .is_err()
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn delete_is_idempotent() {
        let root = temp_root("del");
        let store = FileSecretStore::at(&root);
        store
            .delete_secret(SVC, ACCT)
            .expect("absent delete is fine");
        std::fs::remove_dir_all(&root).ok();
    }
}
