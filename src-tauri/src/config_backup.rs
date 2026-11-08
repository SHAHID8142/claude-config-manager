//! Secure backup of the global settings file (T29).
//!
//! Policy (threat T15, PRD F07/F08):
//! - Backup BEFORE any write; timestamped, unique names; content is a
//!   byte-exact copy of the source (restore must be complete, so plaintext
//!   env secrets stay in the backup — that is why every other property is
//!   locked down below).
//! - Backups live in the app-owned directory with **0600** permissions.
//! - Metadata only in lists (name/size/time) — content never enters logs,
//!   errors, or UI.
//! - Symlinked sources and symlinked entries are refused/skipped (threat T5).

use crate::config_locator::{ConfigLocator, LocatedStatus};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum BackupError {
    MissingSource,
    SymlinkSource,
    Io(String),
}

impl std::fmt::Display for BackupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackupError::MissingSource => write!(f, "nothing to back up — settings file missing"),
            BackupError::SymlinkSource => write!(f, "settings path is a symlink — refused"),
            BackupError::Io(m) => write!(f, "backup failed ({m})"),
        }
    }
}

impl std::error::Error for BackupError {}

pub const BACKUP_PREFIX: &str = "settings-backup-";

/// Strict backup-name contract: exactly `settings-backup-<digits>.json`.
pub fn is_valid_backup_name(name: &str) -> bool {
    let Some(rest) = name.strip_prefix(BACKUP_PREFIX) else {
        return false;
    };
    let Some(digits) = rest.strip_suffix(".json") else {
        return false;
    };
    !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
}

/// Default app-owned backup directory (production path; tests pass their own).
pub fn default_backup_dir(home: &Path) -> PathBuf {
    home.join("Library")
        .join("Application Support")
        .join("ClaudeConfigManager")
        .join("backups")
}

/// Create a 0600 backup of the settings file. Returns the backup path.
pub fn create_backup(config_path: &Path, backup_dir: &Path) -> Result<PathBuf, BackupError> {
    let locator = ConfigLocator::new();
    match locator.inspect(config_path) {
        Ok(LocatedStatus::RegularFile) => {}
        Ok(LocatedStatus::Missing) => return Err(BackupError::MissingSource),
        Ok(LocatedStatus::Symlink) => return Err(BackupError::SymlinkSource),
        Ok(_) => {
            return Err(BackupError::Io(
                "settings path is not a regular file".into(),
            ));
        }
        Err(e) => return Err(BackupError::Io(e.to_string())),
    }

    let content = std::fs::read(config_path).map_err(|e| BackupError::Io(e.to_string()))?;

    std::fs::create_dir_all(backup_dir).map_err(|e| BackupError::Io(e.to_string()))?;

    let name = format!("{BACKUP_PREFIX}{}.json", timestamp_utc());
    let backup_path = backup_dir.join(name);

    write_private(&backup_path, &content)?;

    // Verify the copy is byte-exact before declaring success.
    let written = std::fs::read(&backup_path).map_err(|e| BackupError::Io(e.to_string()))?;
    if written.len() != content.len() {
        return Err(BackupError::Io("backup verification failed (size)".into()));
    }

    Ok(backup_path)
}

/// Write bytes with 0600 permissions and fsync (best-effort durability).
fn write_private(path: &Path, bytes: &[u8]) -> Result<(), BackupError> {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;

    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| BackupError::Io(e.to_string()))?;
    f.write_all(bytes)
        .map_err(|e| BackupError::Io(e.to_string()))?;
    let _ = f.sync_all();
    drop(f);
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(|e| BackupError::Io(e.to_string()))?;
    Ok(())
}

/// UTC nanosecond timestamp — lexicographically sortable, collision-proof
/// within a process and near-collision-proof across processes.
fn timestamp_utc() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub file_name: String,
    pub size_bytes: u64,
    /// Unix epoch seconds.
    pub created_epoch: u64,
}

/// List backups, newest first. Metadata only — never content.
/// Entries that are symlinks are skipped (threat T5).
pub fn list_backups(backup_dir: &Path) -> Result<Vec<BackupInfo>, BackupError> {
    let entries = match std::fs::read_dir(backup_dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(BackupError::Io(e.to_string())),
    };
    let mut out = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let name = entry.file_name().to_string_lossy().to_string();
        // Strict name contract: settings-backup-<digits>.json. Anything else
        // (traversal-shaped, wrong extension, directories) is skipped — a
        // listing can never surface a path-shaped name (T31).
        if !is_valid_backup_name(&name) {
            continue;
        }
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.file_type().is_symlink() || !meta.is_file() {
            continue;
        }
        let created_epoch = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        out.push(BackupInfo {
            file_name: name,
            size_bytes: meta.len(),
            created_epoch,
        });
    }
    out.sort_by(|a, b| b.file_name.cmp(&a.file_name));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ccm-t29-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    const FAKE_DOC: &str = r#"{"env":{"ANTHROPIC_API_KEY":"sk-ant-FAKE-xyz"},"model":"m"}"#;

    #[test]
    fn backup_is_byte_exact_with_0600() {
        let dir = temp_dir("roundtrip");
        let settings = dir.join("settings.json");
        fs::write(&settings, FAKE_DOC).expect("source");
        let backups = dir.join("backups");

        let path = create_backup(&settings, &backups).expect("backup");

        // Byte-exact copy (backup intentionally contains the secret — restore
        // completeness; the policy is 0600 + local-only + never-logged).
        assert_eq!(fs::read(&path).expect("read backup"), FAKE_DOC.as_bytes());

        let mode = fs::metadata(&path).expect("meta").permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "backup must be owner-only");

        fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn backup_names_are_prefixed_and_unique() {
        let dir = temp_dir("names");
        let settings = dir.join("settings.json");
        fs::write(&settings, "{}").expect("source");
        let backups = dir.join("backups");

        let p1 = create_backup(&settings, &backups).expect("b1");
        let p2 = create_backup(&settings, &backups).expect("b2");
        assert_ne!(p1, p2);
        for p in [&p1, &p2] {
            assert!(
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(BACKUP_PREFIX)
            );
        }
        fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn missing_source_refused() {
        let dir = temp_dir("missing");
        let backups = dir.join("backups");
        let err = create_backup(&dir.join("settings.json"), &backups).expect_err("missing");
        assert_eq!(err, BackupError::MissingSource);
        fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn symlinked_source_refused() {
        let dir = temp_dir("symsrc");
        let victim = dir.join("victim.txt");
        fs::write(&victim, "sensitive").expect("victim");
        let link = dir.join("settings.json");
        std::os::unix::fs::symlink(&victim, &link).expect("symlink");
        let err = create_backup(&link, &dir.join("backups")).expect_err("symlink");
        assert_eq!(err, BackupError::SymlinkSource);
        fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn list_is_newest_first_metadata_only() {
        let dir = temp_dir("list");
        let settings = dir.join("settings.json");
        fs::write(&settings, FAKE_DOC).expect("source");
        let backups = dir.join("backups");
        let p1 = create_backup(&settings, &backups).expect("b1");
        let p2 = create_backup(&settings, &backups).expect("b2");

        let infos = list_backups(&backups).expect("list");
        assert_eq!(infos.len(), 2);
        // Newest (larger timestamp) first.
        assert_eq!(
            infos[0].file_name,
            p2.file_name().unwrap().to_string_lossy()
        );
        assert_eq!(
            infos[1].file_name,
            p1.file_name().unwrap().to_string_lossy()
        );
        // Metadata only: the fake secret must not appear anywhere in the list.
        let rendered = format!("{infos:?}");
        assert!(!rendered.contains("sk-ant-FAKE"));
        fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn symlinked_backup_entries_are_skipped() {
        let dir = temp_dir("symentry");
        let backups = dir.join("backups");
        fs::create_dir_all(&backups).expect("backups dir");
        let outside = dir.join("outside.json");
        fs::write(&outside, "{}").expect("outside file");
        let link_name = format!("{BACKUP_PREFIX}9999.json");
        std::os::unix::fs::symlink(&outside, backups.join(&link_name)).expect("symlink entry");

        let infos = list_backups(&backups).expect("list");
        assert!(
            !infos.iter().any(|i| i.file_name == link_name),
            "symlinked entries must be skipped"
        );
        fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn non_backup_files_are_ignored() {
        let dir = temp_dir("filter");
        let backups = dir.join("backups");
        fs::create_dir_all(&backups).expect("backups dir");
        fs::write(backups.join("unrelated.txt"), "x").expect("unrelated");
        fs::write(backups.join("settings-backup-not-json.txt"), "x").expect("wrong ext");
        let infos = list_backups(&backups).expect("list");
        assert_eq!(infos.len(), 0, "only our prefix+suffix matches");
        fs::remove_dir_all(&dir).expect("cleanup");
    }
}
