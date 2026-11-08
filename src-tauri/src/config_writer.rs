//! Atomic settings writer with concurrent-modification detection and
//! rollback (T30).
//!
//! Pipeline (each step fail-closed):
//! 1. refuse symlinked targets (threat T5)
//! 2. concurrent-modification detection: caller snapshots the file's mtime
//!    (via [`snapshot_mtime`]) before reading/merging; if the mtime changed
//!    by write time, the write REFUSES — another process (Claude Code, an
//!    editor) touched the file in between (threat T4).
//! 3. temp file in the SAME directory, 0600, fsync
//! 4. atomic rename over the target (no partial state observable)
//! 5. read-back verification (parse + value equality)
//! 6. on verification failure: rollback restores the original bytes via the
//!    same temp+rename machinery

use crate::config_locator::{ConfigLocator, LocatedStatus};
use crate::config_reader::ConfigError;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Snapshot the target's mtime before reading/merging; pass it back to
/// [`write_settings_atomic`] to detect concurrent edits in between.
pub fn snapshot_mtime(path: &Path) -> Result<Option<SystemTime>, ConfigError> {
    match std::fs::symlink_metadata(path) {
        Ok(m) => Ok(m.modified().ok()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(ConfigError::Io(e.to_string())),
    }
}

/// Serialize with stable pretty formatting (diff-friendly, strict JSON).
fn serialize(doc: &Value) -> Result<String, ConfigError> {
    serde_json::to_string_pretty(doc).map_err(|e| ConfigError::Io(e.to_string()))
}

fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

fn temp_path_for(target: &Path) -> PathBuf {
    let file = target
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "settings.json".to_string());
    target.with_file_name(format!(".{file}.ccm-tmp-{}", now_nanos()))
}

/// Write bytes via temp-file + fsync + atomic rename with 0600 perms.
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), ConfigError> {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;

    let tmp = temp_path_for(path);
    {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
            .map_err(|e| ConfigError::Io(format!("temp file: {}", e)))?;
        f.write_all(bytes)
            .map_err(|e| ConfigError::Io(format!("temp write: {}", e)))?;
        f.sync_all()
            .map_err(|e| ConfigError::Io(format!("temp fsync: {}", e)))?;
    }
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))
        .map_err(|e| ConfigError::Io(format!("temp chmod: {}", e)))?;
    std::fs::rename(&tmp, path).map_err(|e| {
        // Best-effort temp cleanup on rename failure.
        let _ = std::fs::remove_file(&tmp);
        ConfigError::Io(format!("atomic rename: {}", e))
    })
}

/// Rollback: restore the original bytes (kept in memory from step 0).
pub fn rollback_to_bytes(path: &Path, original: &[u8]) -> Result<(), ConfigError> {
    atomic_write(path, original).map_err(|e| ConfigError::Io(format!("rollback failed: {}", e)))
}

/// Atomically write `doc` to `path`.
///
/// `expected_mtime`: the snapshot taken before the merge. `Some(t)` with a
/// mismatching current mtime → `ConcurrentModification`. `None` skips the
/// check (tests / caller-managed locking).
///
/// After a successful rename the file is re-read and parsed; a mismatch
/// triggers rollback to the pre-write bytes.
pub fn write_settings_atomic(
    path: &Path,
    doc: &Value,
    expected_mtime: Option<SystemTime>,
) -> Result<(), ConfigError> {
    let locator = ConfigLocator::new();
    match locator.inspect(path) {
        Ok(LocatedStatus::RegularFile) => {}
        Ok(LocatedStatus::Missing) => return Err(ConfigError::Missing),
        Ok(LocatedStatus::Symlink) => return Err(ConfigError::SymlinkRefused),
        Ok(LocatedStatus::OtherFile) => {
            return Err(ConfigError::Io("not a regular file".to_string()));
        }
        Err(e) => return Err(ConfigError::Io(e.to_string())),
    }

    // Concurrent-modification detection (threat T4).
    if let Some(expected) = expected_mtime {
        let current = std::fs::symlink_metadata(path)
            .map_err(|e| ConfigError::Io(e.to_string()))?
            .modified()
            .ok();
        if current != Some(expected) {
            return Err(ConfigError::ConcurrentModification);
        }
    }

    let original = std::fs::read(path).map_err(|e| ConfigError::Io(e.to_string()))?;
    let new_bytes = serialize(doc)?.into_bytes();

    atomic_write(path, &new_bytes)?;

    // Read-back verification (PRD F07): parse and compare values.
    let verify = std::fs::read_to_string(path)
        .map_err(|e| ConfigError::Io(format!("read-back: {}", e)))
        .and_then(|s| {
            serde_json::from_str::<Value>(&s)
                .map_err(|e| ConfigError::Io(format!("read-back parse: {}", e)))
        });
    match verify {
        Ok(written) if &written == doc => Ok(()),
        Ok(_) => {
            // Verification failed: restore original, then surface the error.
            rollback_to_bytes(path, &original)?;
            Err(ConfigError::Io(
                "write verification failed — original content restored".to_string(),
            ))
        }
        Err(e) => {
            rollback_to_bytes(path, &original)?;
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_reader::read_settings;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::thread::sleep;
    use std::time::Duration;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ccm-t30-{tag}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    #[test]
    fn atomic_write_round_trips_with_0600() {
        let dir = temp_dir("happy");
        let path = dir.join("settings.json");
        fs::write(&path, r#"{"keep":true}"#).expect("seed");

        let doc: Value =
            serde_json::from_str(r#"{"keep":true,"model":"claude-4-5"}"#).expect("doc");
        write_settings_atomic(&path, &doc, None).expect("write");

        let read_back = read_settings(&path).expect("read");
        assert_eq!(read_back, doc);
        let mode = fs::metadata(&path).expect("meta").permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "written settings must be owner-only");

        // No temp residue left behind.
        let residue: Vec<_> = fs::read_dir(&dir)
            .expect("dir")
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains("ccm-tmp"))
            .collect();
        assert!(residue.is_empty(), "temp files must not leak: {residue:?}");
        fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn concurrent_modification_is_detected() {
        let dir = temp_dir("conflict");
        let path = dir.join("settings.json");
        fs::write(&path, r#"{"v":1}"#).expect("seed");

        let mtime = snapshot_mtime(&path).expect("snapshot");
        sleep(Duration::from_millis(20)); // ensure coarse mtime granularity passes
        // Another writer touches the file between snapshot and write.
        fs::write(&path, r#"{"v":2}"#).expect("interloper");

        let doc: Value = serde_json::from_str(r#"{"v":3}"#).expect("doc");
        let err = write_settings_atomic(&path, &doc, mtime).expect_err("must refuse");
        assert_eq!(err, ConfigError::ConcurrentModification);
        // The interloper's content is intact — we did not clobber it.
        assert_eq!(fs::read_to_string(&path).expect("read"), r#"{"v":2}"#);
        fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn mtime_snapshot_none_after_first_write_is_allowed() {
        let dir = temp_dir("fresh");
        let path = dir.join("settings.json");
        // File did not exist at snapshot time.
        let mtime = snapshot_mtime(&path).expect("snapshot");
        assert_eq!(mtime, None);
        fs::write(&path, "{}").expect("seed by CLI");
        // Caller with None (no expectation) writes fine; a Some-based check
        // against a fresh file is the caller's responsibility to refresh.
        let doc: Value = serde_json::from_str("{}").expect("doc");
        write_settings_atomic(&path, &doc, None).expect("write");
        fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn symlink_target_refused() {
        let dir = temp_dir("symlink");
        let victim = dir.join("victim.txt");
        fs::write(&victim, "x").expect("victim");
        let path = dir.join("settings.json");
        std::os::unix::fs::symlink(&victim, &path).expect("symlink");
        let doc: Value = serde_json::from_str("{}").expect("doc");
        assert_eq!(
            write_settings_atomic(&path, &doc, None),
            Err(ConfigError::SymlinkRefused)
        );
        fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn missing_target_refused_at_write_time() {
        let dir = temp_dir("missing");
        let path = dir.join("settings.json");
        let doc: Value = serde_json::from_str("{}").expect("doc");
        assert_eq!(
            write_settings_atomic(&path, &doc, None),
            Err(ConfigError::Missing)
        );
        fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn rollback_restores_original_bytes() {
        let dir = temp_dir("rollback");
        let path = dir.join("settings.json");
        let original = r#"{"original":true}"#;
        fs::write(&path, original).expect("seed");
        let trashed: Value = serde_json::from_str(r#"{"trashed":1}"#).expect("doc");
        fs::write(&path, r#"{"trashed":1}"#).expect("simulate bad write");

        rollback_to_bytes(&path, original.as_bytes()).expect("rollback");
        assert_eq!(fs::read_to_string(&path).expect("read"), original);
        let _ = trashed; // silence unused in non-assert path
        fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn no_temp_residue_after_refused_write() {
        let dir = temp_dir("residue");
        let path = dir.join("settings.json");
        fs::write(&path, "{}").expect("seed");
        let mtime = snapshot_mtime(&path).expect("snapshot");
        sleep(Duration::from_millis(20));
        fs::write(&path, "{}").expect("touch");
        let doc: Value = serde_json::from_str("{}").expect("doc");
        let _ = write_settings_atomic(&path, &doc, mtime);
        let residue: Vec<_> = fs::read_dir(&dir)
            .expect("dir")
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains("ccm-tmp"))
            .collect();
        assert!(residue.is_empty(), "refused writes must not leave temps");
        fs::remove_dir_all(&dir).expect("cleanup");
    }
}
