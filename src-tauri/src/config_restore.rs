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
use std::path::Path;

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
                write!(f, "selected backup is not valid JSON ({m}) — not restored")
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

/// Redacted backup metadata suitable for a recovery preview. Authentication
/// values and helper paths/content never cross this boundary.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupPreview {
    pub file_name: String,
    pub size_bytes: u64,
    pub created_epoch: u64,
    pub valid: bool,
    pub detail: String,
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub helper_present: bool,
}

fn check_backup_directory(backups_dir: &Path) -> Result<(), RestoreError> {
    for dir in [Some(backups_dir), backups_dir.parent()]
        .into_iter()
        .flatten()
    {
        match std::fs::symlink_metadata(dir) {
            Ok(meta) if meta.file_type().is_symlink() || !meta.is_dir() => {
                return Err(RestoreError::Io(
                    "backup directory must be a regular directory, not a symlink".into(),
                ));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(RestoreError::Io(e.to_string())),
        }
    }
    Ok(())
}

fn backup_entries(
    backups_dir: &Path,
) -> Result<Vec<crate::config_backup::BackupInfo>, RestoreError> {
    check_backup_directory(backups_dir)?;
    list_backups(backups_dir).map_err(RestoreError::Backup)
}

/// Recheck the directory/name/file at use time. The open itself refuses final
/// symlinks on the supported macOS/Linux targets; a capped read also protects
/// against a file growing after its metadata was inspected.
fn read_backup(backups_dir: &Path, file_name: &str) -> Result<Value, RestoreError> {
    use std::io::Read;
    if !crate::config_backup::is_valid_backup_name(file_name) {
        return Err(RestoreError::Io(
            "choose a backup listed by this app".into(),
        ));
    }
    check_backup_directory(backups_dir)?;
    let path = backups_dir.join(file_name);
    let meta = std::fs::symlink_metadata(&path).map_err(|e| RestoreError::Io(e.to_string()))?;
    if !meta.is_file() || meta.file_type().is_symlink() {
        return Err(RestoreError::Io("backup is not a regular file".into()));
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        use std::os::unix::fs::OpenOptionsExt;
        #[cfg(target_os = "macos")]
        const NOFOLLOW: i32 = 0x100;
        #[cfg(target_os = "linux")]
        const NOFOLLOW: i32 = 0x20000;
        options.custom_flags(NOFOLLOW);
    }
    let file = options
        .open(path)
        .map_err(|e| RestoreError::Io(e.to_string()))?;
    let opened_meta = file
        .metadata()
        .map_err(|e| RestoreError::Io(e.to_string()))?;
    let limit = crate::config_reader::SETTINGS_MAX_BYTES;
    if !opened_meta.is_file() || opened_meta.len() > limit {
        return Err(RestoreError::InvalidBackupContent(
            "not a regular JSON object within the settings size limit".into(),
        ));
    }
    let mut raw = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut raw)
        .map_err(|e| RestoreError::Io(e.to_string()))?;
    if raw.len() as u64 > limit {
        return Err(RestoreError::InvalidBackupContent(
            "backup exceeds the settings size limit".into(),
        ));
    }
    let doc: Value = serde_json::from_slice(&raw).map_err(|e| {
        RestoreError::InvalidBackupContent(format!("line {} column {}", e.line(), e.column()))
    })?;
    if !doc.is_object() {
        return Err(RestoreError::InvalidBackupContent(
            "top level is not an object".into(),
        ));
    }
    Ok(doc)
}

fn selected_backup(
    backups_dir: &Path,
    file_name: Option<&str>,
    skip_invalid: bool,
) -> Result<(String, Value), RestoreError> {
    let entries = backup_entries(backups_dir)?;
    if let Some(name) = file_name {
        if !crate::config_backup::is_valid_backup_name(name)
            || !entries.iter().any(|entry| entry.file_name == name)
        {
            return Err(RestoreError::Io(
                "selected backup is unavailable; refresh the backup list".into(),
            ));
        }
        return Ok((name.into(), read_backup(backups_dir, name)?));
    }
    for entry in entries {
        match read_backup(backups_dir, &entry.file_name) {
            Ok(doc) => return Ok((entry.file_name, doc)),
            Err(_) if skip_invalid => continue,
            Err(error) => return Err(error),
        }
    }
    Err(RestoreError::NoBackups)
}

fn credential_values(doc: &Value) -> Vec<&str> {
    doc.get("env")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|env| env.iter())
        .filter(|(key, _)| {
            let key = key.to_ascii_uppercase();
            key.contains("KEY") || key.contains("TOKEN") || key.contains("SECRET")
        })
        .filter_map(|(_, value)| value.as_str())
        .filter(|value| !value.is_empty())
        .collect()
}

fn preview_model(doc: &Value, secrets: &[&str]) -> Option<String> {
    let value = doc
        .pointer("/env/ANTHROPIC_MODEL")
        .and_then(Value::as_str)
        .or_else(|| doc.get("model").and_then(Value::as_str))?;
    if value.len() > 256
        || value.chars().any(char::is_control)
        || crate::redact::has_credential_prefix(value)
        || secrets.iter().any(|secret| value.contains(secret))
    {
        return None;
    }
    // Opaque token-shaped strings have no useful preview. Dated Claude IDs
    // remain readable; general log heuristics otherwise mistake them for keys.
    if crate::redact::is_secret_shaped(value) && !value.starts_with("claude-") {
        return None;
    }
    Some(value.to_owned())
}

fn preview_endpoint(doc: &Value, secrets: &[&str]) -> Option<String> {
    let raw = doc.pointer("/env/ANTHROPIC_BASE_URL")?.as_str()?;
    if raw.len() > 4096 {
        return None;
    }
    let mut url = url::Url::parse(raw).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    url.set_username("").ok()?;
    url.set_password(None).ok()?;
    url.set_query(None);
    url.set_fragment(None);
    let value = url.to_string();
    if secrets.iter().any(|secret| value.contains(secret))
        || url
            .path_segments()
            .into_iter()
            .flatten()
            .any(crate::redact::is_secret_shaped)
        || url
            .host_str()
            .is_some_and(crate::redact::has_credential_prefix)
    {
        return None;
    }
    Some(value)
}

/// Newest-first validated previews. Invalid files remain visible but cannot be
/// selected for restore. Returned details are fixed messages, never JSON content.
pub fn list_backup_previews(backups_dir: &Path) -> Result<Vec<BackupPreview>, RestoreError> {
    backup_entries(backups_dir)?
        .into_iter()
        .map(|entry| {
            let mut preview = BackupPreview {
                file_name: entry.file_name,
                size_bytes: entry.size_bytes,
                created_epoch: entry.created_epoch,
                valid: false,
                detail: "Backup cannot be read as a valid JSON object.".into(),
                model: None,
                base_url: None,
                helper_present: false,
            };
            if let Ok(doc) = read_backup(backups_dir, &preview.file_name) {
                preview.valid = true;
                preview.detail =
                    "Valid JSON object. Provider connectivity has not been checked.".into();
                let secrets = credential_values(&doc);
                preview.model = preview_model(&doc, &secrets);
                preview.base_url = preview_endpoint(&doc, &secrets);
                preview.helper_present = doc
                    .get("apiKeyHelper")
                    .and_then(Value::as_str)
                    .and_then(crate::credential_handoff::helper_command_path)
                    .and_then(|path| std::fs::symlink_metadata(path).ok())
                    .is_some_and(|meta| meta.is_file() && !meta.file_type().is_symlink());
            }
            Ok(preview)
        })
        .collect()
}

/// Preserve current bytes. Invalid originals are explicitly kept outside the
/// restorable backup namespace, so a recovery does not poison the next restore.
fn preserve_current(settings_path: &Path, backups_dir: &Path) -> Result<String, RestoreError> {
    let preserved = create_backup(settings_path, backups_dir).map_err(RestoreError::Backup)?;
    let name = preserved.file_name().unwrap().to_string_lossy().to_string();
    if crate::config_reader::read_settings(settings_path).is_err() {
        let invalid = name.replace("settings-backup-", "invalid-settings-");
        std::fs::rename(&preserved, backups_dir.join(&invalid))
            .map_err(|e| RestoreError::Io(e.to_string()))?;
        Ok(invalid)
    } else {
        Ok(name)
    }
}

/// Compatibility wrapper: newest ordinary backup, refusing a corrupt newest.
pub fn restore_last_backup(
    settings_path: &Path,
    backups_dir: &Path,
) -> Result<RestoreOutcome, RestoreError> {
    restore_backup(settings_path, backups_dir, None)
}

/// Restore an explicitly listed backup, or newest ordinary backup when omitted.
pub fn restore_backup(
    settings_path: &Path,
    backups_dir: &Path,
    file_name: Option<&str>,
) -> Result<RestoreOutcome, RestoreError> {
    check_backup_directory(backups_dir)?;
    let _lock = crate::apply_lock::ApplyLock::acquire(backups_dir.parent().unwrap_or(backups_dir))
        .map_err(|e| RestoreError::Io(e.to_string()))?;
    let expected = crate::config_writer::snapshot_mtime(settings_path)
        .map_err(|e| RestoreError::Io(e.to_string()))?;
    let (name, doc) = selected_backup(backups_dir, file_name, false)?;
    let pre_restore = match std::fs::symlink_metadata(settings_path) {
        Ok(meta) if meta.is_file() && !meta.file_type().is_symlink() => {
            preserve_current(settings_path, backups_dir)?
        }
        Ok(_) => {
            return Err(RestoreError::Io(
                "settings path is not a regular file".into(),
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(RestoreError::Io(error.to_string())),
    };
    crate::config_writer::write_settings_atomic_checked(settings_path, &doc, expected)
        .map_err(|e| RestoreError::Io(e.to_string()))?;
    Ok(RestoreOutcome {
        restored_from: name,
        pre_restore_backup: pre_restore,
    })
}

pub fn recover_settings(
    settings_path: &Path,
    backups_dir: &Path,
    use_backup: bool,
) -> Result<RestoreOutcome, RestoreError> {
    recover_settings_selected(settings_path, backups_dir, use_backup, None)
}

/// Explicit recovery of invalid settings from a selected valid backup or clean
/// object. A missing/valid current file cannot accidentally enter this path.
pub fn recover_settings_selected(
    settings_path: &Path,
    backups_dir: &Path,
    use_backup: bool,
    file_name: Option<&str>,
) -> Result<RestoreOutcome, RestoreError> {
    check_backup_directory(backups_dir)?;
    let _lock = crate::apply_lock::ApplyLock::acquire(backups_dir.parent().unwrap_or(backups_dir))
        .map_err(|e| RestoreError::Io(e.to_string()))?;
    let expected = crate::config_writer::snapshot_mtime(settings_path)
        .map_err(|e| RestoreError::Io(e.to_string()))?;
    match crate::config_reader::read_settings(settings_path) {
        Ok(_) => {
            return Err(RestoreError::Io(
                "settings are already valid; refresh before recovering".into(),
            ));
        }
        Err(
            ConfigError::MalformedJson(_)
            | ConfigError::MalformedJsonHint(_)
            | ConfigError::MalformedShape(_),
        ) => {}
        Err(error) => return Err(RestoreError::CurrentStateUnreadable(error)),
    }
    let (name, doc) = if use_backup {
        selected_backup(backups_dir, file_name, true)?
    } else if file_name.is_some() {
        return Err(RestoreError::Io(
            "choose either a backup or clean settings".into(),
        ));
    } else {
        ("clean settings".into(), serde_json::json!({}))
    };
    let archive_name = preserve_current(settings_path, backups_dir)?;
    crate::config_writer::write_settings_atomic_checked(settings_path, &doc, expected)
        .map_err(|e| RestoreError::Io(e.to_string()))?;
    Ok(RestoreOutcome {
        restored_from: name,
        pre_restore_backup: archive_name,
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
