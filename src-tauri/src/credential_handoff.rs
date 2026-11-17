//! Secure credential handoff to Claude Code via `apiKeyHelper` (T32).
//!
//! Verified facts (T02 matrix + 2026-11-08 doc checks):
//! - `apiKeyHelper` runs "a shell script that returns an API key"; output on
//!   stdout (empty output/timeout/error ⇒ request failure within three
//!   attempts); >10 s runtime triggers a UI warning — our script is
//!   stateless and takes ~10 ms, so cadence is irrelevant.
//! - Helper credentials rank BELOW `ANTHROPIC_AUTH_TOKEN`/`ANTHROPIC_API_KEY`
//!   in auth precedence ⇒ helper mode and env-key mode are MUTUALLY
//!   EXCLUSIVE (enforced in the merge layer, see config_merge).
//! - Works for the CLI and the VS Code extension (the PRD's primary user
//!   context); not for Claude Desktop/cloud sessions.
//!
//! Security design (threat T1/T6):
//! - The helper script is APP-GENERATED with a fixed template; the only
//!   interpolated value is the provider id, strictly validated
//!   (`[a-z0-9-]{1,64}`) before embedding — no shell injection surface.
//! - The script reads the credential from an owner-only file (0600) in the
//!   app's `~/.claude-config-manager/` directory; the secret NEVER appears in
//!   settings.json, in the script, or in any error/log (F08). See
//!   `file_secret_store` for why the Keychain was dropped.
//! - Installed into the app-owned helpers directory with 0700 dir / 0700
//!   script permissions, written atomically (temp+rename).

use crate::config_backup::BackupError;
use crate::secret_store::SecretError;
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static HELPER_TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct PendingHelper(PathBuf);
impl Drop for PendingHelper {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Check only app-owned components. macOS /tmp, /var and a user's home may be
/// legitimate symlink ancestors and are deliberately outside this boundary.
fn check_helper_directories(helpers_dir: &Path) -> Result<(), BackupError> {
    let owned_root = helpers_dir
        .ancestors()
        .find(|p| p.file_name().is_some_and(|n| n == ".claude-config-manager"));
    for directory in helpers_dir.ancestors() {
        match std::fs::symlink_metadata(directory) {
            Ok(meta) if !meta.is_dir() || meta.file_type().is_symlink() => {
                return Err(BackupError::Io(
                    "helper directory is not an owned regular directory".into(),
                ));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(BackupError::Io(e.to_string())),
        }
        if owned_root.is_none() || Some(directory) == owned_root {
            break;
        }
    }
    Ok(())
}

fn private_directory_mode(directory: &Path) -> Result<(), BackupError> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(target_os = "macos")]
    options.custom_flags(0x100 | 0x4); // O_NOFOLLOW | O_NONBLOCK
    #[cfg(target_os = "linux")]
    options.custom_flags(0x20000 | 0x800); // O_NOFOLLOW | O_NONBLOCK
    let file = options
        .open(directory)
        .map_err(|e| BackupError::Io(e.to_string()))?;
    if !file
        .metadata()
        .map_err(|e| BackupError::Io(e.to_string()))?
        .is_dir()
    {
        return Err(BackupError::Io(
            "helper directory is not a directory".into(),
        ));
    }
    file.set_permissions(std::fs::Permissions::from_mode(0o700))
        .map_err(|e| BackupError::Io(e.to_string()))
}

/// Encode one executable path for Claude Code's /bin/sh command line.
/// Ordinary paths retain their old representation; metacharacters are quoted.
pub fn helper_command(path: &Path) -> String {
    let raw = path.to_string_lossy();
    if raw
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "/._-".contains(c))
    {
        raw.into_owned()
    } else {
        format!("'{}'", raw.replace('\'', "'\\''"))
    }
}
/// Recognize only our single-path encoding, never arbitrary shell commands.
pub fn helper_command_path(command: &str) -> Option<PathBuf> {
    if command.len() > 4096 || command.chars().any(char::is_control) {
        return None;
    }
    let raw = if command.starts_with('\'') && command.ends_with('\'') && command.len() >= 2 {
        command[1..command.len() - 1].replace("'\\''", "'")
    } else {
        command.to_owned()
    };
    let path = PathBuf::from(raw);
    (path.is_absolute() && helper_command(&path) == command).then_some(path)
}

/// Strict provider-id validation shared with the catalog ([a-z0-9-]{1,64}).
pub fn validate_provider_id(provider_id: &str) -> Result<(), SecretError> {
    // T59: must start with a letter/digit. A leading '-' would make the id
    // look like a command-line flag to any consumer that forwards it, even
    // though our own `security` invocation quotes it as a value.
    let mut chars = provider_id.chars();
    let starts_safely = chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    let ok = !provider_id.is_empty()
        && provider_id.len() <= 64
        && starts_safely
        && provider_id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if ok {
        Ok(())
    } else {
        Err(SecretError::InvalidIdentifier)
    }
}

/// The fixed helper-script template. Only `provider_id` is interpolated and
/// it is validated by [`validate_provider_id`] at every entry point.
pub fn generate_helper_script(provider_id: &str) -> Result<String, SecretError> {
    generate_helper_script_for(provider_id, provider_id)
}

/// V2-21: helper for a *specific* credential file.
///
/// `account` is the credential id (`openrouter-1`) and becomes the secret file
/// stem; `provider_id` only labels the script and names the helper file. Both
/// are validated, so neither can escape `secrets/`.
pub fn generate_helper_script_for(provider_id: &str, account: &str) -> Result<String, SecretError> {
    validate_provider_id(provider_id)?;
    crate::credential_index::validate_credential_id(account)
        .map_err(|_| SecretError::InvalidIdentifier)?;
    // The path is DERIVED, never user input: `$HOME` is expanded by the shell at
    // request time, and the id is validated above, so there is no injection
    // surface. `cat` fails closed (non-zero, no output) when the file is missing,
    // so Claude Code treats it as an authentication failure instead of sending a
    // blank credential.
    Ok(format!(
        r#"#!/bin/sh
# Generated by Claude Config Manager — do not edit.
# Prints the stored credential for provider '{provider_id}'. The secret lives in
# an owner-only file (0600), never in settings.json and never in this script.
f="$HOME/.claude-config-manager/secrets/{account}.secret"
[ ! -L "$HOME/.claude-config-manager" ] && [ ! -L "$HOME/.claude-config-manager/secrets" ] && [ ! -L "$f" ] || {{ echo "credential path is a symlink; refusing" >&2; exit 1; }}
[ -f "$f" ] || {{ echo "credential file missing for {account}" >&2; exit 1; }}
# Portable permission check: GNU stat first (Linux), BSD stat fallback (macOS).
perms="$(stat -c %a "$f" 2>/dev/null || stat -f %Lp "$f" 2>/dev/null)"
[ "$perms" = "600" ] || {{ echo "credential file permissions are too open: $f" >&2; exit 1; }}
exec cat "$f"
"#
    ))
}

/// Default app-owned helpers directory (production; tests inject their own).
pub fn default_helpers_dir(home: &Path) -> PathBuf {
    // NO SPACES in this path. Claude Code runs the `apiKeyHelper` value itself
    // (not through a quoted shell invocation), so `~/Library/Application Support`
    // was truncated at the space and the helper failed with exit 127:
    //   /bin/sh: /Users/…/Library/Application: No such file or directory
    app_root_dir(home).join("helpers")
}

/// App-owned data directory whose path contains no spaces (see above).
pub fn app_root_dir(home: &Path) -> PathBuf {
    home.join(".claude-config-manager")
}

/// Install (or atomically replace) the helper script for a provider.
/// Returns the absolute script path to store in `apiKeyHelper`.
pub fn install_helper_script(
    helpers_dir: &Path,
    provider_id: &str,
    script: &str,
) -> Result<PathBuf, BackupError> {
    validate_provider_id(provider_id).map_err(|_| BackupError::Io("invalid provider id".into()))?;

    check_helper_directories(helpers_dir)?;
    std::fs::create_dir_all(helpers_dir).map_err(|e| BackupError::Io(e.to_string()))?;
    check_helper_directories(helpers_dir)?;
    if let Some(root) = helpers_dir
        .ancestors()
        .find(|p| p.file_name().is_some_and(|n| n == ".claude-config-manager"))
    {
        private_directory_mode(root)?;
    }
    private_directory_mode(helpers_dir)?;

    let target = helpers_dir.join(format!("{provider_id}.sh"));
    if std::fs::symlink_metadata(&target).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(BackupError::Io(
            "helper path is a symlink; refusing to replace it".into(),
        ));
    }
    let tmp = helpers_dir.join(format!(
        ".{provider_id}.sh.tmp-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        HELPER_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o700)
        .open(&tmp)
        .map_err(|e| BackupError::Io(e.to_string()))?;
    // The guard exists only once create_new succeeds, so an EEXIST collision
    // never removes a temporary file belonging to another installer.
    let pending = PendingHelper(tmp);
    file.set_permissions(std::fs::Permissions::from_mode(0o700))
        .map_err(|e| BackupError::Io(e.to_string()))?;
    file.write_all(script.as_bytes())
        .map_err(|e| BackupError::Io(e.to_string()))?;
    file.sync_all()
        .map_err(|e| BackupError::Io(e.to_string()))?;
    std::fs::rename(&pending.0, &target).map_err(|e| BackupError::Io(e.to_string()))?;
    if let Ok(directory) = std::fs::File::open(helpers_dir) {
        let _ = directory.sync_all();
    }

    // Verify installed state before returning the path.
    let mode = file
        .metadata()
        .map_err(|e| BackupError::Io(e.to_string()))?
        .permissions()
        .mode();
    if mode & 0o777 != 0o700 {
        return Err(BackupError::Io("helper script permissions wrong".into()));
    }
    Ok(target)
}

/// Remove the helper script for a provider. Idempotent.
pub fn uninstall_helper_script(helpers_dir: &Path, provider_id: &str) -> Result<(), BackupError> {
    validate_provider_id(provider_id).map_err(|_| BackupError::Io("invalid provider id".into()))?;
    check_helper_directories(helpers_dir)?;
    let target = helpers_dir.join(format!("{provider_id}.sh"));
    match std::fs::remove_file(&target) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(BackupError::Io(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_is_fixed_shape_with_validated_id() {
        let s = generate_helper_script("anthropic").expect("script");
        assert!(s.starts_with("#!/bin/sh"));
        assert!(s.contains(".claude-config-manager/secrets/anthropic.secret"));
        assert!(
            !s.contains("Application Support"),
            "no spaces allowed in the path"
        );
        // Fails closed rather than emitting an empty credential.
        assert!(s.contains("exit 1"));
        // Refuses a file whose permissions were loosened.
        assert!(s.contains("stat -c %a")); // GNU/Linux first
        assert!(s.contains("stat -f %Lp")); // BSD/macOS fallback
        // Idempotent: same id → identical script.
        assert_eq!(s, generate_helper_script("anthropic").expect("again"));
    }

    #[test]
    fn script_never_contains_secret_material() {
        // Generation takes NO secret input — prove it by scanning.
        let s = generate_helper_script("openrouter").expect("script");
        assert!(!s.contains("sk-"));
        assert!(!s.contains("nvapi-"));
    }

    #[test]
    fn adversarial_provider_ids_rejected_before_embedding() {
        for bad in [
            "",                                                                           // empty
            "Anthropic",  // uppercase
            "with space", // space
            "semi;colon", // shell metachar
            "back`tick",  // shell substitution
            "$(danger)",  // command substitution
            "../../etc",  // path traversal
            "a-very-long-id-that-exceeds-the-sixty-four-character-limit-by-a-lot-more-x", // >64
        ] {
            assert!(
                matches!(
                    generate_helper_script(bad),
                    Err(SecretError::InvalidIdentifier)
                ),
                "must reject {bad:?}"
            );
        }
    }

    #[test]
    fn install_sets_0700_and_absolute_path() {
        let dir = std::env::temp_dir().join(format!(
            "ccm-t32-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let script = generate_helper_script("deepseek").expect("script");
        let path = install_helper_script(&dir, "deepseek", &script).expect("install");

        assert!(path.is_absolute());
        assert!(path.ends_with("deepseek.sh"));
        let mode = std::fs::metadata(&path).expect("meta").permissions().mode();
        assert_eq!(mode & 0o777, 0o700, "script must be 0700");
        let dir_mode = std::fs::metadata(&dir).expect("dmeta").permissions().mode();
        assert_eq!(dir_mode & 0o777, 0o700, "helpers dir must be 0700");
        assert!(
            std::fs::read_to_string(&path)
                .expect("read")
                .starts_with("#!/bin/sh")
        );

        // Atomic replace: install again → same path, still valid.
        let again = install_helper_script(&dir, "deepseek", &script).expect("reinstall");
        assert_eq!(path, again);

        uninstall_helper_script(&dir, "deepseek").expect("uninstall");
        assert!(!path.exists());
        // Idempotent second uninstall.
        uninstall_helper_script(&dir, "deepseek").expect("uninstall again");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_temp_residue_after_install() {
        let dir = std::env::temp_dir().join(format!(
            "ccm-t32-res-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let script = generate_helper_script("zai").expect("script");
        install_helper_script(&dir, "zai", &script).expect("install");
        let residue: Vec<_> = std::fs::read_dir(&dir)
            .expect("dir")
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp-"))
            .collect();
        assert!(residue.is_empty(), "temp files must not leak: {residue:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
