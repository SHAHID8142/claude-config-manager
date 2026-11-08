//! `/usr/bin/security`-backed store (fallback for T-keychain/G11).
//!
//! Why this exists: the primary store uses the Security framework through the
//! `keyring` crate. In some environments macOS refuses that call with a platform
//! error (reported as "keychain access refused by the OS") and Save & Apply died
//! with it. The `security` CLI writes and reads the *same* logical item, and it
//! is the same mechanism the generated `apiKeyHelper` script uses — so falling
//! back to it keeps write and read paths consistent.
//!
//! Secret handling: the value is passed to `security` on **stdin** (`-w` without
//! an argument prompts for it twice), never on the command line, so it cannot be
//! read from `ps`. Helper arguments carry only the service and account names,
//! which are already public (they are visible in `settings.json`).

use crate::secret_store::{SecretError, SecretStore};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Exit code macOS uses for "item not found" (`errSecItemNotFound`).
const ITEM_NOT_FOUND_EXIT: i32 = 44;

#[derive(Debug, Clone)]
pub struct SecurityCliStore {
    security: PathBuf,
}

impl Default for SecurityCliStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityCliStore {
    pub fn new() -> Self {
        // Absolute path: a hostile PATH must not be able to answer for us.
        Self {
            security: PathBuf::from("/usr/bin/security"),
        }
    }

    #[cfg(test)]
    fn with_binary(path: &std::path::Path) -> Self {
        Self {
            security: path.to_path_buf(),
        }
    }

    fn validate(service: &str, account: &str) -> Result<(), SecretError> {
        for (name, value) in [("service", service), ("account", account)] {
            if value.is_empty()
                || value.len() > 255
                || value.starts_with('-')
                || value.chars().any(|c| c.is_control())
            {
                return Err(SecretError::InvalidIdentifier);
            }
            let _ = name;
        }
        Ok(())
    }

    fn redact(stderr: &str) -> String {
        let first = stderr.lines().next().unwrap_or("").trim();
        if first.is_empty() {
            "security command failed".to_string()
        } else {
            // `security` never echoes the password; keep the line short anyway.
            first.chars().take(120).collect()
        }
    }
}

impl SecretStore for SecurityCliStore {
    fn set_secret(&self, service: &str, account: &str, secret: &str) -> Result<(), SecretError> {
        Self::validate(service, account)?;
        let mut child = Command::new(&self.security)
            .args([
                "add-generic-password",
                "-U", // update in place when the item already exists
                "-a",
                account,
                "-s",
                service,
                "-w", // no value: read the password from stdin (twice)
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| SecretError::Platform(format!("could not run security ({e})")))?;

        {
            let stdin = child
                .stdin
                .as_mut()
                .ok_or_else(|| SecretError::Platform("security stdin unavailable".into()))?;
            // Prompted twice: "password data" then "retype password".
            writeln!(stdin, "{secret}").map_err(|e| SecretError::Platform(e.to_string()))?;
            writeln!(stdin, "{secret}").map_err(|e| SecretError::Platform(e.to_string()))?;
        }

        let out = child
            .wait_with_output()
            .map_err(|e| SecretError::Platform(e.to_string()))?;
        if out.status.success() {
            return Ok(());
        }
        Err(SecretError::Platform(Self::redact(
            &String::from_utf8_lossy(&out.stderr),
        )))
    }

    fn get_secret(&self, service: &str, account: &str) -> Result<Option<String>, SecretError> {
        Self::validate(service, account)?;
        let out = Command::new(&self.security)
            .args(["find-generic-password", "-a", account, "-s", service, "-w"])
            .stdin(Stdio::null())
            .output()
            .map_err(|e| SecretError::Platform(format!("could not run security ({e})")))?;
        if out.status.success() {
            let value = String::from_utf8_lossy(&out.stdout).trim_end().to_string();
            return Ok(Some(value));
        }
        if out.status.code() == Some(ITEM_NOT_FOUND_EXIT) {
            return Ok(None);
        }
        Err(SecretError::Platform(Self::redact(
            &String::from_utf8_lossy(&out.stderr),
        )))
    }

    fn delete_secret(&self, service: &str, account: &str) -> Result<(), SecretError> {
        Self::validate(service, account)?;
        let out = Command::new(&self.security)
            .args(["delete-generic-password", "-a", account, "-s", service])
            .stdin(Stdio::null())
            .output()
            .map_err(|e| SecretError::Platform(format!("could not run security ({e})")))?;
        if out.status.success() || out.status.code() == Some(ITEM_NOT_FOUND_EXIT) {
            return Ok(());
        }
        Err(SecretError::Platform(Self::redact(
            &String::from_utf8_lossy(&out.stderr),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;
    use std::time::SystemTime;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ccm-g11-{tag}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("dir");
        dir
    }

    /// Fake `security` that records its argv and the secret it received on stdin.
    fn fake_security(dir: &Path, exit: i32) -> PathBuf {
        let path = dir.join("security");
        fs::write(
            &path,
            format!(
                "#!/bin/sh\n\
                 printf '%s' \"$*\" > {dir}/argv.txt\n\
                 if [ {exit} -eq 0 ]; then cat > {dir}/stdin.txt; fi\n\
                 if [ {exit} -eq 44 ]; then echo 'security: item not found' >&2; fi\n\
                 exit {exit}\n",
                dir = dir.display(),
                exit = exit
            ),
        )
        .expect("write fake");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).expect("chmod");
        path
    }

    /// The secret must never appear on argv (visible in `ps` to any process).
    #[test]
    fn secret_is_passed_on_stdin_not_on_the_command_line() {
        let dir = temp_dir("argv");
        let store = SecurityCliStore::with_binary(&fake_security(&dir, 0));
        store
            .set_secret(
                "com.claude-config-manager.deepseek",
                "deepseek",
                "sk-ant-SECRET-G11",
            )
            .expect("set");

        let argv = fs::read_to_string(dir.join("argv.txt")).expect("argv");
        assert!(
            !argv.contains("sk-ant-SECRET-G11"),
            "secret leaked into argv: {argv}"
        );
        assert!(argv.contains("add-generic-password"));
        assert!(argv.contains("-s com.claude-config-manager.deepseek"));

        let stdin = fs::read_to_string(dir.join("stdin.txt")).expect("stdin");
        assert!(
            stdin.contains("sk-ant-SECRET-G11"),
            "secret must arrive on stdin, got {stdin:?}"
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_item_maps_to_none_not_an_error() {
        let dir = temp_dir("missing");
        let store = SecurityCliStore::with_binary(&fake_security(&dir, 44));
        assert_eq!(
            store
                .get_secret("com.claude-config-manager.x", "x")
                .expect("ok"),
            None
        );
        store
            .delete_secret("com.claude-config-manager.x", "x")
            .expect("delete is idempotent");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn failures_surface_the_platform_message_without_the_secret() {
        let dir = temp_dir("fail");
        let store = SecurityCliStore::with_binary(&fake_security(&dir, 1));
        let err = store
            .set_secret("com.claude-config-manager.x", "x", "sk-ant-SECRET-G11")
            .expect_err("must fail");
        let msg = err.to_string();
        assert!(!msg.contains("sk-ant-SECRET-G11"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn hostile_identifiers_are_refused_before_spawning_anything() {
        let dir = temp_dir("ids");
        let store = SecurityCliStore::with_binary(&fake_security(&dir, 0));
        for (svc, acct) in [
            ("-x", "a"),
            ("ok", "-a"),
            ("bad\nservice", "a"),
            ("", "a"),
            (&"s".repeat(300), "a"),
        ] {
            assert!(
                store.set_secret(svc, acct, "v").is_err(),
                "identifier {svc:?}/{acct:?} must be refused"
            );
        }
        assert!(
            !dir.join("argv.txt").exists(),
            "nothing may be spawned for invalid identifiers"
        );
        fs::remove_dir_all(&dir).ok();
    }
}
