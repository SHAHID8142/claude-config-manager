//! Restore the most recent settings backup (T35, PRD F07 rollback support).
//!
//! Safety pipeline:
//! 1. newest backup via the strict T29 listing (symlinks/traversal skipped)
//! 2. backup content validated as strict-JSON object BEFORE touching anything
//!    (a corrupted backup is never restored)
//! 3. a **pre-restore safety backup** of the current settings is taken first
//!    (restore is itself a write and must be reversible)
//! 4. write via the T30 atomic writer (0600, fsync, conflict check, read-back)

use crate::config_backup::{BackupError, create_backup, list_backups};
use crate::config_reader::ConfigError;
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum RestoreError {
    NoBackups,
    InvalidBackupContent(String),
    CurrentStateUnreadable(ConfigError),
    Backup(BackupError),
    Io(String),
}

impl std::fmt::Display for RestoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RestoreError::NoBackups => write!(f, "no backups available to restore"),
            RestoreError::InvalidBackupContent(m) => {
                write!(f, "newest backup is not valid JSON ({m}) — not restored")
            }
            RestoreError::CurrentStateUnreadable(e) => {
                write!(f, "current settings unreadable, refusing to restore: {e}")
            }
            RestoreError::Backup(e) => write!(f, "{e}"),
            RestoreError::Io(m) => write!(f, "restore failed ({m})"),
        }
    }
}

impl std::error::Error for RestoreError {}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreOutcome {
    pub restored_from: String,
    pub pre_restore_backup: String,
}

/// Restore the newest backup over the current settings file.
pub fn restore_last_backup(
    settings_path: &Path,
    backups_dir: &Path,
) -> Result<RestoreOutcome, RestoreError> {
    // 1. newest valid backup entry
    let backups = list_backups(backups_dir).map_err(RestoreError::Backup)?;
    let newest = backups
        .first()
        .ok_or(RestoreError::NoBackups)?
        .file_name
        .clone();

    // 2. content gate BEFORE any write
    let backup_path = PathBuf::from(backups_dir).join(&newest);
    let raw = std::fs::read(&backup_path).map_err(|e| RestoreError::Io(e.to_string()))?;
    let doc: Value = serde_json::from_slice(&raw).map_err(|e| {
        RestoreError::InvalidBackupContent(format!("line {} column {}", e.line(), e.column()))
    })?;
    if !doc.is_object() {
        return Err(RestoreError::InvalidBackupContent(
            "top level is not an object".into(),
        ));
    }

    // 3. pre-restore safety backup of the CURRENT state (only if one exists)
    let pre_restore = match std::fs::symlink_metadata(settings_path) {
        Ok(m) if m.is_file() => create_backup(settings_path, backups_dir)
            .map_err(RestoreError::Backup)?
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default(),
        Ok(_) => {
            return Err(RestoreError::Io(
                "settings path is not a regular file".into(),
            ));
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(RestoreError::Io(e.to_string())),
    };

    // 4. atomic write of the backup content (conflict detection via fresh mtime)
    let expected = crate::config_writer::snapshot_mtime(settings_path)
        .map_err(|e| RestoreError::Io(e.to_string()))?;
    crate::config_writer::write_settings_atomic(settings_path, &doc, expected)
        .map_err(|e| RestoreError::Io(e.to_string()))?;

    Ok(RestoreOutcome {
        restored_from: newest,
        pre_restore_backup: pre_restore,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::thread::sleep;
    use std::time::Duration;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ccm-t35-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    #[test]
    fn restores_newest_backup_with_pre_restore_safety_copy() {
        let dir = temp_dir("happy");
        let settings = dir.join("settings.json");
        let backups = dir.join("backups");

        // History: good state backed up, then the file regressed.
        fs::write(&settings, r#"{"good":"state-1"}"#).expect("v1");
        sleep(Duration::from_millis(15));
        let b1 = create_backup(&settings, &backups).expect("backup1");

        fs::write(&settings, r#"{"bad":"regression"}"#).expect("regress");
        sleep(Duration::from_millis(15));
        let b2 = create_backup(&settings, &backups).expect("backup2");
        fs::write(&settings, r#"{"worse":true}"#).expect("worse state");

        // Restore should pick the NEWEST backup (b2), not b1.
        let out = restore_last_backup(&settings, &backups).expect("restore");
        assert_eq!(out.restored_from, b2.file_name().unwrap().to_string_lossy());
        let restored: Value =
            serde_json::from_str(&fs::read_to_string(&settings).expect("read")).expect("json");
        assert_eq!(restored, serde_json::json!({"bad": "regression"}));
        // Pre-restore backup captured the "worse" state.
        let pre = backups.join(&out.pre_restore_backup);
        assert_eq!(
            fs::read_to_string(&pre).expect("pre-restore content"),
            r#"{"worse":true}"#
        );
        // Pre-restore backup is 0600.
        let mode = fs::metadata(&pre).expect("meta").permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
        let _ = b1; // earlier backup still present
        fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn no_backups_refused_cleanly() {
        let dir = temp_dir("none");
        let settings = dir.join("settings.json");
        fs::write(&settings, "{}").expect("current");
        let err = restore_last_backup(&settings, &dir.join("backups")).expect_err("no backups");
        assert_eq!(err, RestoreError::NoBackups);
        // Current state untouched.
        assert_eq!(fs::read_to_string(&settings).expect("read"), "{}");
        fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn corrupted_backup_never_restored() {
        let dir = temp_dir("corrupt");
        let settings = dir.join("settings.json");
        let backups = dir.join("backups");
        fs::create_dir_all(&backups).expect("backups dir");
        // A truncated backup that still passes the strict name contract.
        fs::write(
            backups.join("settings-backup-1700000000000000000.json"),
            r#"{"broken": tru"#,
        )
        .expect("corrupt backup");
        fs::write(&settings, r#"{"current":true}"#).expect("current");

        let err = restore_last_backup(&settings, &backups).expect_err("corrupt");
        assert!(matches!(err, RestoreError::InvalidBackupContent(_)));
        // Current state untouched.
        assert_eq!(
            fs::read_to_string(&settings).expect("read"),
            r#"{"current":true}"#
        );
        fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn non_object_backup_refused() {
        let dir = temp_dir("array");
        let settings = dir.join("settings.json");
        let backups = dir.join("backups");
        fs::create_dir_all(&backups).expect("backups dir");
        fs::write(
            backups.join("settings-backup-1700000000000000001.json"),
            "[1,2,3]",
        )
        .expect("array backup");
        fs::write(&settings, "{}").expect("current");
        let err = restore_last_backup(&settings, &backups).expect_err("array");
        assert!(matches!(err, RestoreError::InvalidBackupContent(_)));
        fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn restore_after_settings_deleted_recreates_file() {
        let dir = temp_dir("recreate");
        let settings = dir.join("settings.json");
        let backups = dir.join("backups");
        fs::write(&settings, r#"{"saved":true}"#).expect("state");
        create_backup(&settings, &backups).expect("backup");
        fs::remove_file(&settings).expect("simulate loss");

        let out = restore_last_backup(&settings, &backups).expect("restore");
        // No pre-restore backup (nothing existed to protect).
        assert_eq!(out.pre_restore_backup, "");
        let restored: Value =
            serde_json::from_str(&fs::read_to_string(&settings).expect("read")).expect("json");
        assert_eq!(restored, serde_json::json!({"saved": true}));
        fs::remove_dir_all(&dir).expect("cleanup");
    }
}
