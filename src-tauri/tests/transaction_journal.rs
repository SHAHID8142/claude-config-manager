use claude_config_manager_lib::transaction_journal::{
    ManagedTransaction, journal_path, pending, recover,
};
use serde_json::Value;
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "ccm-journal-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(path.join(".claude")).unwrap();
        fs::create_dir_all(path.join(".claude-config-manager/helpers")).unwrap();
        fs::create_dir_all(path.join(".claude-config-manager/secrets")).unwrap();
        Self(path)
    }
    fn settings(&self) -> PathBuf {
        self.0.join(".claude/settings.json")
    }
    fn owned(&self, p: &str) -> PathBuf {
        self.0.join(".claude-config-manager").join(p)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn private_write(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

#[test]
fn dropped_operation_blocks_new_changes_and_explicit_recovery_restores_every_before_image() {
    let f = Fixture::new();
    private_write(&f.settings(), b"{\"model\":\"old-model\"}");
    private_write(
        &f.owned("credentials.json"),
        b"{\"version\":1,\"credentials\":[]}",
    );
    private_write(&f.owned("secrets/old.secret"), b"private-old-fixture");
    fs::write(f.owned("helpers/old.sh"), b"#!/bin/sh\nprintf old").unwrap();
    let mut tx = ManagedTransaction::begin(&f.0, Some(&f.settings())).unwrap();
    let added = f.owned("secrets/added.secret");
    tx.track(&added).unwrap();
    fs::write(&f.settings(), b"{\"model\":\"new-model\"}").unwrap();
    fs::write(f.owned("secrets/old.secret"), b"private-new-fixture").unwrap();
    fs::write(f.owned("helpers/old.sh"), b"broken helper").unwrap();
    fs::write(&added, b"new key").unwrap();
    drop(tx);
    let summary = serde_json::to_string(&pending(&f.0).unwrap().unwrap()).unwrap();
    assert!(!summary.contains("private-old-fixture"));
    assert!(!summary.contains("bytesBase64"));
    assert!(!summary.contains("old.secret"));
    assert!(ManagedTransaction::begin(&f.0, Some(&f.settings())).is_err());
    let result = recover(&f.0, Some(&f.settings()), true).unwrap();
    assert!(result.restored_previous);
    let archive = f.owned(result.current_snapshot_file.as_ref().unwrap());
    assert!(archive.is_file());
    assert_eq!(
        fs::metadata(archive).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        fs::read(&f.settings()).unwrap(),
        b"{\"model\":\"old-model\"}"
    );
    assert_eq!(
        fs::read(f.owned("secrets/old.secret")).unwrap(),
        b"private-old-fixture"
    );
    assert_eq!(
        fs::read(f.owned("helpers/old.sh")).unwrap(),
        b"#!/bin/sh\nprintf old"
    );
    assert!(!added.exists());
    assert_eq!(
        fs::metadata(f.owned("helpers/old.sh"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(f.owned("secrets/old.secret"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert!(pending(&f.0).unwrap().is_none());
}
#[test]
fn commit_keeps_new_state_and_durably_clears_marker() {
    let f = Fixture::new();
    private_write(&f.settings(), b"{}");
    let tx = ManagedTransaction::begin(&f.0, Some(&f.settings())).unwrap();
    fs::write(f.settings(), b"{\"model\":\"new\"}").unwrap();
    tx.commit().unwrap();
    assert!(pending(&f.0).unwrap().is_none());
    assert!(fs::read_to_string(f.settings()).unwrap().contains("new"));
}
#[test]
fn rollback_failure_is_reported_and_marker_remains_retryable() {
    let f = Fixture::new();
    private_write(&f.settings(), b"{}");
    let mut tx = ManagedTransaction::begin(&f.0, Some(&f.settings())).unwrap();
    let new = f.owned("secrets/new.secret");
    tx.track(&new).unwrap();
    fs::create_dir(&new).unwrap();
    let error = tx.rollback().unwrap_err();
    assert!(error.contains("incomplete"));
    assert!(journal_path(&f.0).is_file());
    fs::remove_dir(new).unwrap();
    recover(&f.0, Some(&f.settings()), true).unwrap();
    assert!(pending(&f.0).unwrap().is_none());
}
#[test]
fn before_missing_file_is_removed_on_handled_failure_and_original_bytes_survive() {
    let f = Fixture::new();
    let settings = f.settings();
    let mut tx = ManagedTransaction::begin(&f.0, Some(&settings)).unwrap();
    let helper = f.owned("helpers/new.sh");
    tx.track(&helper).unwrap();
    fs::write(&settings, b"{}").unwrap();
    fs::write(&helper, b"new script").unwrap();
    tx.rollback().unwrap();
    assert!(!settings.exists());
    assert!(!helper.exists());
}
#[test]
fn traversal_foreign_scope_and_wrong_settings_recovery_are_refused() {
    let f = Fixture::new();
    private_write(&f.settings(), b"{}");
    let mut tx = ManagedTransaction::begin(&f.0, Some(&f.settings())).unwrap();
    assert!(tx.track(&f.owned("secrets/../../outside.secret")).is_err());
    assert!(tx.track(&f.0.join("unmanaged.txt")).is_err());
    drop(tx);
    assert!(recover(&f.0, Some(&f.0.join("elsewhere/settings.json")), true).is_err());
    assert!(journal_path(&f.0).exists());
    let path = journal_path(&f.0);
    let mut doc: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    doc["files"][0]["path"] = Value::String(f.0.join("unmanaged.txt").to_string_lossy().into());
    private_write(&path, &serde_json::to_vec(&doc).unwrap());
    assert!(recover(&f.0, Some(&f.settings()), true).is_err());
    assert!(pending(&f.0).is_err());
}
#[test]
fn symlink_sources_destinations_and_journal_are_refused_without_touching_outside_data() {
    let f = Fixture::new();
    let outside = f.0.join("outside");
    private_write(&outside, b"outside secret");
    symlink(&outside, f.owned("secrets/bad.secret")).unwrap();
    assert!(ManagedTransaction::begin(&f.0, None).is_err());
    assert_eq!(fs::read(&outside).unwrap(), b"outside secret");
    fs::remove_file(f.owned("secrets/bad.secret")).unwrap();
    private_write(&f.settings(), b"{}");
    let tx = ManagedTransaction::begin(&f.0, Some(&f.settings())).unwrap();
    drop(tx);
    fs::remove_file(f.settings()).unwrap();
    symlink(&outside, f.settings()).unwrap();
    assert!(recover(&f.0, Some(&f.settings()), true).is_err());
    assert_eq!(fs::read(&outside).unwrap(), b"outside secret");
    fs::remove_file(f.settings()).unwrap();
    fs::remove_file(journal_path(&f.0)).unwrap();
    symlink(&outside, journal_path(&f.0)).unwrap();
    assert!(pending(&f.0).is_err());
}
#[test]
fn metadata_only_journal_and_explicit_keep_current_do_not_restore_old_settings() {
    let f = Fixture::new();
    private_write(
        &f.owned("routing.json"),
        b"{\"version\":1,\"enabled\":false,\"targets\":[]}",
    );
    let tx = ManagedTransaction::begin(&f.0, None).unwrap();
    drop(tx);
    fs::write(
        f.owned("routing.json"),
        b"{\"version\":1,\"enabled\":true,\"targets\":[]}",
    )
    .unwrap();
    let result = recover(&f.0, None, false).unwrap();
    assert!(!result.restored_previous);
    assert!(result.current_snapshot_file.is_none());
    assert!(
        fs::read_to_string(f.owned("routing.json"))
            .unwrap()
            .contains("true")
    );
}
#[test]
fn invalid_current_json_cannot_be_kept_but_can_be_restored() {
    let f = Fixture::new();
    private_write(&f.settings(), b"{}");
    let tx = ManagedTransaction::begin(&f.0, Some(&f.settings())).unwrap();
    fs::write(f.settings(), b"invalid").unwrap();
    drop(tx);
    assert!(recover(&f.0, Some(&f.settings()), false).is_err());
    assert!(journal_path(&f.0).exists());
    recover(&f.0, Some(&f.settings()), true).unwrap();
    assert_eq!(fs::read(f.settings()).unwrap(), b"{}");
}
#[test]
fn parent_symlink_is_refused_while_legitimate_system_temp_alias_is_allowed() {
    let f = Fixture::new();
    let outside = f.0.join("outside");
    fs::create_dir_all(&outside).unwrap();
    let nested = f.0.join("config-alias");
    symlink(&outside, &nested).unwrap();
    let path = nested.join("deep/settings.json");
    assert!(ManagedTransaction::begin(&f.0, Some(&path)).is_err());
    assert!(!outside.join("deep").exists());
}
#[test]
fn child_exit_without_destructors_leaves_a_durable_recoverable_journal() {
    const ENV: &str = "CCM_JOURNAL_CRASH_FIXTURE";
    if let Some(home) = std::env::var_os(ENV) {
        let home = PathBuf::from(home);
        let settings = home.join(".claude/settings.json");
        let _tx = ManagedTransaction::begin(&home, Some(&settings)).unwrap();
        fs::write(settings, b"{\"after\":true}").unwrap();
        std::process::exit(71);
    }
    let f = Fixture::new();
    private_write(&f.settings(), b"{\"before\":true}");
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "child_exit_without_destructors_leaves_a_durable_recoverable_journal",
            "--nocapture",
        ])
        .env(ENV, &f.0)
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(71));
    assert!(pending(&f.0).unwrap().is_some());
    // ApplyLock intentionally protects a just-created empty lock for five
    // seconds; wait out that documented grace before reclaiming the dead owner.
    std::thread::sleep(Duration::from_secs(6));
    recover(&f.0, Some(&f.settings()), true).unwrap();
    assert_eq!(fs::read(f.settings()).unwrap(), b"{\"before\":true}");
}

#[test]
fn failed_apply_compensates_owned_files_without_overwriting_external_settings() {
    let f = Fixture::new();
    private_write(&f.settings(), b"{}");
    private_write(&f.owned("secrets/old.secret"), b"original-fixture");
    let tx = ManagedTransaction::begin(&f.0, Some(&f.settings())).unwrap();
    private_write(&f.settings(), b"{\"editor\":true}");
    private_write(&f.owned("secrets/old.secret"), b"changed-fixture");
    tx.rollback_preserving_settings(false).unwrap();
    assert_eq!(fs::read(f.settings()).unwrap(), b"{\"editor\":true}");
    assert_eq!(
        fs::read(f.owned("secrets/old.secret")).unwrap(),
        b"original-fixture"
    );
    assert!(pending(&f.0).unwrap().is_none());
}

#[test]
fn live_transaction_excludes_a_second_process() {
    const MODE: &str = "CCM_TRANSACTION_CONTENTION_FIXTURE";
    if let Some(home) = std::env::var_os(MODE) {
        let result = ManagedTransaction::begin(Path::new(&home), None);
        assert!(result.err().unwrap().contains("another Save & Apply"));
        return;
    }
    let f = Fixture::new();
    let tx = ManagedTransaction::begin(&f.0, Some(&f.settings())).unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "live_transaction_excludes_a_second_process",
            "--nocapture",
        ])
        .env(MODE, &f.0)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    tx.rollback().unwrap();
}
