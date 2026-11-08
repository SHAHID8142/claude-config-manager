//! Consolidated adversarial path tests (T31): symlink (file AND config-dir),
//! path-traversal shape, and permission-denied across the read/backup/write
//! pipeline. These complement the per-module unit tests with cross-stage
//! coverage on shared fixtures.

use claude_config_manager_lib::config_backup::{create_backup, list_backups};
use claude_config_manager_lib::config_locator::{ConfigLocator, LocatedStatus};
use claude_config_manager_lib::config_reader::{ConfigError, read_settings};
use claude_config_manager_lib::config_writer::{rollback_to_bytes, write_settings_atomic};
use serde_json::Value;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::PathBuf;
use std::time::SystemTime;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ccm-t31-{tag}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("temp dir");
    dir
}

fn fixture_home(tag: &str) -> PathBuf {
    let home = temp_dir(tag);
    fs::create_dir_all(home.join(".claude")).expect("config dir");
    fs::write(
        home.join(".claude/settings.json"),
        r#"{"model":"seed","outside":{"stay":true}}"#,
    )
    .expect("seed settings");
    home
}

#[test]
fn symlinked_settings_file_refused_at_every_stage() {
    let home = fixture_home("file-symlink");
    let victim = home.join("innocent.txt");
    fs::write(&victim, "sensitive").expect("victim");
    let settings = home.join(".claude/settings.json");
    fs::remove_file(&settings).expect("remove seed");
    symlink(&victim, &settings).expect("symlink");

    let locator = ConfigLocator::new();
    let path = locator.locate(&home, None);

    assert_eq!(
        locator.inspect(&path).expect("inspect"),
        LocatedStatus::Symlink
    );
    assert_eq!(read_settings(&path), Err(ConfigError::SymlinkRefused));
    assert_eq!(
        create_backup(&path, &home.join("backups")),
        Err(claude_config_manager_lib::config_backup::BackupError::SymlinkSource)
    );
    let doc: Value = serde_json::from_str("{}").expect("doc");
    assert_eq!(
        write_settings_atomic(&path, &doc, None),
        Err(ConfigError::SymlinkRefused)
    );

    // The victim file is untouched — nothing read or written through the link.
    assert_eq!(fs::read_to_string(&victim).expect("victim"), "sensitive");
    fs::remove_dir_all(&home).expect("cleanup");
}

#[test]
fn symlinked_config_dir_refused_for_read_and_write() {
    let home = fixture_home("dir-symlink");
    let real_dir = temp_dir("dir-symlink-target");
    fs::write(real_dir.join("settings.json"), r#"{"redirected":true}"#).expect("target");

    let claude_dir = home.join(".claude");
    fs::remove_dir_all(&claude_dir).expect("remove real dir");
    symlink(&real_dir, &claude_dir).expect("symlink config dir");

    let settings = home.join(".claude/settings.json");
    assert_eq!(read_settings(&settings), Err(ConfigError::SymlinkRefused));
    let doc: Value = serde_json::from_str("{}").expect("doc");
    assert_eq!(
        write_settings_atomic(&settings, &doc, None),
        Err(ConfigError::SymlinkRefused)
    );

    // Target dir never received a write through the redirect.
    assert_eq!(
        fs::read_to_string(real_dir.join("settings.json")).expect("target"),
        r#"{"redirected":true}"#
    );
    fs::remove_dir_all(&home).expect("cleanup");
    fs::remove_dir_all(&real_dir).expect("cleanup");
}

#[test]
fn backup_listing_ignores_path_traversal_shaped_entries() {
    let home = fixture_home("traversal");
    let backups = home.join("backups");
    fs::create_dir_all(&backups).expect("backups dir");

    // Adversarially-shaped entries cannot pass the strict name contract:
    // `..` inside the name (created without OS-level traversal) and a
    // directory wearing our name are both skipped by list_backups.
    fs::write(backups.join("settings-backup-1..2.json"), "x").expect("dotdot name");
    fs::write(backups.join("settings-backup-1;2.json"), "x").expect("semicolon name");
    let _ = fs::create_dir_all(backups.join("settings-backup-dir.json"));
    // One legitimate entry for the positive control.
    fs::write(
        backups.join("settings-backup-1700000000000000000.json"),
        "{}",
    )
    .expect("legit entry");

    let infos = list_backups(&backups).expect("list");
    assert_eq!(infos.len(), 1, "only the strict-pattern entry survives");
    assert_eq!(
        infos[0].file_name,
        "settings-backup-1700000000000000000.json"
    );
    for info in &infos {
        assert!(!info.file_name.contains(".."));
    }
    fs::remove_dir_all(&home).expect("cleanup");
}

#[test]
fn permission_denied_file_read_reports_perm() {
    let home = fixture_home("perm-read");
    let settings = home.join(".claude/settings.json");
    fs::set_permissions(&settings, fs::Permissions::from_mode(0o000)).expect("chmod");
    let result = read_settings(&settings);
    fs::set_permissions(&settings, fs::Permissions::from_mode(0o644)).expect("restore");
    match result {
        Err(ConfigError::PermissionDenied(_)) => {}
        other => panic!("expected PermissionDenied, got {other:?}"),
    }
    fs::remove_dir_all(&home).expect("cleanup");
}

#[test]
fn permission_denied_backup_dir_reports_io_not_panic() {
    let home = fixture_home("perm-backup");
    let backups = home.join("backups");
    fs::create_dir_all(&backups).expect("backups dir");
    fs::set_permissions(&backups, fs::Permissions::from_mode(0o500)).expect("chmod r-x");
    let result = create_backup(&home.join(".claude/settings.json"), &backups);
    fs::set_permissions(&backups, fs::Permissions::from_mode(0o700)).expect("restore");
    assert!(result.is_err(), "unwritable backup dir must fail cleanly");
    fs::remove_dir_all(&home).expect("cleanup");
}

#[test]
fn permission_denied_write_target_rolls_back_cleanly() {
    let home = fixture_home("perm-write");
    let settings = home.join(".claude/settings.json");
    let doc: Value = serde_json::from_str(r#"{"model":"new"}"#).expect("doc");

    // Make the config dir read-only: rename() into it fails → clean Io error.
    let claude_dir = home.join(".claude");
    fs::set_permissions(&claude_dir, fs::Permissions::from_mode(0o500)).expect("chmod r-x");
    let result = write_settings_atomic(&settings, &doc, None);
    fs::set_permissions(&claude_dir, fs::Permissions::from_mode(0o700)).expect("restore");

    assert!(result.is_err(), "read-only config dir must fail cleanly");
    // Original content intact.
    assert!(
        fs::read_to_string(&settings)
            .expect("read")
            .contains("seed")
    );
    // No temp residue.
    let residue: Vec<_> = fs::read_dir(&claude_dir)
        .expect("dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains("ccm-tmp"))
        .collect();
    assert!(residue.is_empty());
    fs::remove_dir_all(&home).expect("cleanup");
}

#[test]
fn rollback_recovers_from_interrupted_write_state() {
    let home = fixture_home("interrupted");
    let settings = home.join(".claude/settings.json");
    let original = fs::read(&settings).expect("original bytes");

    // Simulate an interrupted write: garbage landed in the file.
    fs::write(&settings, b"{\"partial\": tru").expect("interrupted content");

    rollback_to_bytes(&settings, &original).expect("rollback");
    assert_eq!(fs::read(&settings).expect("restored"), original);
    fs::remove_dir_all(&home).expect("cleanup");
}
