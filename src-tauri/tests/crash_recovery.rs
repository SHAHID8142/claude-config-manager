//! T61 — config rollback and interrupted-write recovery.
//!
//! Simulates the states a crash can leave behind (orphan temp file, failed
//! rename, concurrent editor) and proves the live `settings.json` is never
//! observed partially written or corrupted.

use claude_config_manager_lib::config_reader::read_settings;
use claude_config_manager_lib::config_restore::{RestoreError, restore_last_backup};
use claude_config_manager_lib::config_writer::{rollback_to_bytes, write_settings_atomic};
use serde_json::{Value, json};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::SystemTime;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ccm-t61-{tag}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("temp dir");
    dir
}

fn orphans(dir: &PathBuf) -> Vec<String> {
    fs::read_dir(dir)
        .expect("read dir")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.contains("ccm-tmp"))
        .collect()
}

#[test]
fn orphan_temp_from_a_crash_is_ignored_and_harmless() {
    let dir = temp_dir("orphan");
    let path = dir.join("settings.json");
    fs::write(&path, r#"{"model":"survivor"}"#).expect("seed");

    // A crash after the temp write but before the rename leaves debris.
    let orphan = dir.join(".settings.json.ccm-tmp-123456789");
    fs::write(&orphan, "{\"model\":\"half-writ").expect("orphan");

    // The live file is what we read — never the temp.
    let doc = read_settings(&path).expect("parse live file");
    assert_eq!(doc["model"], "survivor");

    // A later apply still works and replaces the live file atomically.
    write_settings_atomic(&path, &json!({"model":"fresh"}), None).expect("write");
    let doc = read_settings(&path).expect("re-read");
    assert_eq!(doc["model"], "fresh");
    assert!(
        orphan.exists(),
        "app does not delete user-visible debris it did not create"
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn interrupted_write_leaves_no_temp_and_keeps_the_original() {
    let dir = temp_dir("interrupted");
    let path = dir.join("settings.json");
    let original = br#"{"model":"original","env":{"KEEP":"1"}}"#;
    fs::write(&path, original).expect("seed");
    let before = fs::read(&path).expect("read");

    // Make the directory unwritable so temp creation fails mid-pipeline.
    let mut perms = fs::metadata(&dir).expect("meta").permissions();
    perms.set_mode(0o500);
    fs::set_permissions(&dir, perms).expect("chmod ro");

    let err = write_settings_atomic(&path, &json!({"model":"never"}), None);
    // Restore write permission before asserting on the filesystem again.
    let mut perms = fs::metadata(&dir).expect("meta").permissions();
    perms.set_mode(0o700);
    fs::set_permissions(&dir, perms).expect("chmod rw");

    assert!(err.is_err(), "unwritable directory must fail the write");
    let after = fs::read(&path).expect("read after");
    assert_eq!(before, after, "original bytes must survive a failed write");
    assert!(
        orphans(&dir).is_empty(),
        "no temp file may be left behind: {:?}",
        orphans(&dir)
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn concurrent_edit_between_snapshot_and_write_is_refused() {
    let dir = temp_dir("concurrent");
    let path = dir.join("settings.json");
    fs::write(&path, r#"{"model":"a"}"#).expect("seed");
    let snapshot = fs::metadata(&path).expect("meta").modified().ok();

    // Another editor changes the file after our snapshot.
    std::thread::sleep(std::time::Duration::from_millis(20));
    fs::write(&path, r#"{"model":"b","edited_by":"other"}"#).expect("other edit");

    let err =
        write_settings_atomic(&path, &json!({"model":"ours"}), snapshot).expect_err("must refuse");
    assert_eq!(
        err,
        claude_config_manager_lib::config_reader::ConfigError::ConcurrentModification
    );
    let doc: Value = serde_json::from_str(&fs::read_to_string(&path).expect("read")).expect("json");
    assert_eq!(
        doc["model"], "b",
        "the other editor's write must not be clobbered"
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn rollback_restores_exact_bytes_with_owner_only_permissions() {
    let dir = temp_dir("rollback");
    let path = dir.join("settings.json");
    let original = br#"{"model":"keep","env":{"A":"1"}}"#;
    fs::write(&path, original).expect("seed");

    // Simulate a bad write, then roll back to the pre-write bytes.
    write_settings_atomic(&path, &json!({"model":"bad"}), None).expect("bad write");
    rollback_to_bytes(&path, original).expect("rollback");

    assert_eq!(fs::read(&path).expect("read"), original);
    let mode = fs::metadata(&path).expect("meta").permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "restored config must be owner-only");
    // And it is still valid, usable JSON.
    assert_eq!(read_settings(&path).expect("parse")["model"], "keep");
    assert!(orphans(&dir).is_empty());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn restore_refuses_a_corrupted_newest_backup_without_touching_settings() {
    let dir = temp_dir("restore-bad");
    let settings = dir.join("settings.json");
    let backups = dir.join("backups");
    fs::create_dir_all(&backups).expect("backups dir");
    fs::write(&settings, r#"{"model":"live"}"#).expect("seed");

    // Newest backup is corrupt (crash while writing it).
    fs::write(backups.join("settings-backup-2.json"), "{not json").expect("corrupt backup");
    fs::write(backups.join("settings-backup-1.json"), r#"{"model":"old"}"#).expect("ok backup");

    let before = fs::read(&settings).expect("read");
    let files_before = fs::read_dir(&backups).expect("dir").count();

    let err = restore_last_backup(&settings, &backups).expect_err("corrupt newest refused");
    assert!(
        matches!(err, RestoreError::InvalidBackupContent(_)),
        "got {err:?}"
    );

    assert_eq!(
        fs::read(&settings).expect("read"),
        before,
        "live file untouched"
    );
    assert_eq!(
        fs::read_dir(&backups).expect("dir").count(),
        files_before,
        "no pre-restore backup may be created for a refused restore"
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn restore_recovers_a_valid_backup_after_a_crash_damaged_settings() {
    let dir = temp_dir("restore-ok");
    let settings = dir.join("settings.json");
    let backups = dir.join("backups");
    fs::create_dir_all(&backups).expect("backups dir");
    fs::write(&settings, "{half-written-by-crash").expect("damaged live file");
    fs::write(
        backups.join("settings-backup-1.json"),
        r#"{"model":"good"}"#,
    )
    .expect("backup");

    let outcome = restore_last_backup(&settings, &backups).expect("restore");
    assert_eq!(outcome.restored_from, "settings-backup-1.json");
    assert_eq!(read_settings(&settings).expect("parse")["model"], "good");
    // The damaged state is preserved as a pre-restore safety copy (never lost).
    assert!(!outcome.pre_restore_backup.is_empty());
    assert!(backups.join(&outcome.pre_restore_backup).exists());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn no_backups_present_is_reported_clearly() {
    let dir = temp_dir("no-backups");
    let settings = dir.join("settings.json");
    fs::write(&settings, r#"{"model":"x"}"#).expect("seed");
    let empty = dir.join("backups");
    fs::create_dir_all(&empty).expect("dir");

    let err = restore_last_backup(&settings, &empty).expect_err("none");
    assert!(matches!(err, RestoreError::NoBackups), "got {err:?}");
    fs::remove_dir_all(&dir).ok();
}
