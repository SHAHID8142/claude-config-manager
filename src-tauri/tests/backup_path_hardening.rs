use claude_config_manager_lib::{
    config_backup::{BackupError, create_backup},
    config_locator::ConfigLocator,
    config_reader::{ConfigError, SETTINGS_MAX_BYTES, read_settings},
    config_writer::{rollback_to_bytes, snapshot_mtime, write_settings_atomic_checked},
};
use serde_json::json;
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn in_dir(parent: &Path) -> Self {
        let p = parent.join(format!(
            "ccm-path-hardening-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&p).unwrap();
        Self(p)
    }
    fn new() -> Self {
        Self::in_dir(&std::env::temp_dir())
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn oversized_backup_source_is_refused_before_creating_any_backup() {
    let f = Fixture::new();
    let source = f.0.join("settings.json");
    let backups = f.0.join("backups");
    fs::File::create(&source)
        .unwrap()
        .set_len(SETTINGS_MAX_BYTES + 1)
        .unwrap();
    assert!(create_backup(&source, &backups).is_err());
    assert!(!backups.exists());
    assert_eq!(fs::metadata(source).unwrap().len(), SETTINGS_MAX_BYTES + 1);
}
#[test]
fn private_backup_preserves_exact_invalid_bytes_with_regular_source() {
    let f = Fixture::new();
    let source = f.0.join("settings.json");
    fs::write(&source, b"invalid\0raw bytes\n").unwrap();
    let backup = create_backup(&source, &f.0.join("backups")).unwrap();
    assert_eq!(fs::read(&backup).unwrap(), b"invalid\0raw bytes\n");
    assert_eq!(
        fs::metadata(backup).unwrap().permissions().mode() & 0o777,
        0o600
    );
}
#[test]
fn nested_symlinked_config_ancestor_refuses_reads_backups_writes_and_rollback() {
    let f = Fixture::new();
    let outside = f.0.join("outside");
    fs::create_dir_all(outside.join("real-parent")).unwrap();
    let actual = outside.join("real-parent/settings.json");
    fs::write(&actual, "{\"before\":42}").unwrap();
    let link = f.0.join("redirect");
    symlink(&outside, &link).unwrap();
    let path = link.join("real-parent/settings.json");
    assert!(ConfigLocator::new().config_dir_is_symlink(&path).unwrap());
    assert_eq!(read_settings(&path), Err(ConfigError::SymlinkRefused));
    assert_eq!(
        create_backup(&path, &f.0.join("backups")),
        Err(BackupError::SymlinkSource)
    );
    assert_eq!(
        write_settings_atomic_checked(
            &path,
            &json!({"after":true}),
            snapshot_mtime(&path).unwrap()
        ),
        Err(ConfigError::SymlinkRefused)
    );
    assert!(rollback_to_bytes(&path, b"{}").is_err());
    assert_eq!(fs::read_to_string(actual).unwrap(), "{\"before\":42}");
    assert_eq!(
        fs::read_dir(outside.join("real-parent")).unwrap().count(),
        1
    );
}
#[test]
fn nested_symlink_is_checked_even_when_inner_parent_does_not_exist_yet() {
    let f = Fixture::new();
    let outside = f.0.join("outside");
    fs::create_dir(&outside).unwrap();
    let link = f.0.join("redirect");
    symlink(&outside, &link).unwrap();
    let path = link.join("missing/inner/settings.json");
    assert_eq!(
        write_settings_atomic_checked(&path, &json!({}), None),
        Err(ConfigError::SymlinkRefused)
    );
    assert!(!outside.join("missing").exists());
}
#[test]
fn symlinked_backup_destination_ancestor_cannot_write_outside_owned_tree() {
    let f = Fixture::new();
    let source = f.0.join("settings.json");
    fs::write(&source, "{}").unwrap();
    let outside = f.0.join("outside");
    fs::create_dir(&outside).unwrap();
    let link = f.0.join("redirect");
    symlink(&outside, &link).unwrap();
    assert!(create_backup(&source, &link.join("nested/backups")).is_err());
    assert_eq!(fs::read_dir(outside).unwrap().count(), 0);
}
#[test]
fn first_run_missing_directories_and_normal_tmp_aliases_remain_usable() {
    let f = Fixture::in_dir(Path::new("/tmp"));
    let source = f.0.join("missing/nested/settings.json");
    write_settings_atomic_checked(&source, &json!({"model":"fixture"}), None).unwrap();
    assert_eq!(read_settings(&source).unwrap()["model"], "fixture");
    let backup = create_backup(&source, &f.0.join("backups")).unwrap();
    assert_eq!(fs::read(backup).unwrap(), fs::read(&source).unwrap());
    assert_eq!(
        fs::metadata(source).unwrap().permissions().mode() & 0o777,
        0o600
    );
}
#[test]
fn new_oversized_document_refuses_before_writing_or_leaving_temporary_files() {
    let f = Fixture::new();
    let path = f.0.join("settings.json");
    fs::write(&path, "{}").unwrap();
    let doc = json!({"payload":"x".repeat(SETTINGS_MAX_BYTES as usize)});
    let result = write_settings_atomic_checked(&path, &doc, snapshot_mtime(&path).unwrap());
    assert!(matches!(result, Err(ConfigError::TooLarge { .. })));
    assert_eq!(fs::read_to_string(&path).unwrap(), "{}");
    assert_eq!(fs::read_dir(&f.0).unwrap().count(), 1);
}
#[test]
fn nonregular_and_final_symlink_sources_are_refused_without_creating_backups() {
    let f = Fixture::new();
    let source = f.0.join("directory.json");
    fs::create_dir(&source).unwrap();
    assert!(create_backup(&source, &f.0.join("backups")).is_err());
    let target = f.0.join("target.json");
    fs::write(&target, "{}").unwrap();
    let link = f.0.join("settings.json");
    symlink(&target, &link).unwrap();
    assert_eq!(
        create_backup(&link, &f.0.join("backups")),
        Err(BackupError::SymlinkSource)
    );
    assert!(!f.0.join("backups").exists());
}
