use claude_config_manager_lib::{
    apply_lock::ApplyLock,
    config_reader::{SETTINGS_MAX_BYTES, read_settings},
    config_restore::{
        list_backup_previews, recover_settings_selected, restore_backup, restore_last_backup,
    },
};
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    root: PathBuf,
    settings: PathBuf,
    backups: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "ccm-selected-backup-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(root.join("backups")).unwrap();
        Self {
            settings: root.join("settings.json"),
            backups: root.join("backups"),
            root,
        }
    }
    fn backup(&self, name: &str, raw: &str) {
        fs::write(self.backups.join(name), raw).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
const OLD: &str = "settings-backup-1700000000000000000.json";
const NEW: &str = "settings-backup-1800000000000000000.json";

#[test]
fn previews_show_validity_and_safe_model_endpoint_without_auth_or_helper_path() {
    let f = Fixture::new();
    let helper = f.root.join("helper.sh");
    fs::write(&helper, "#!/bin/sh\n").unwrap();
    let secret = "sk-ant-fixture-sensitive-123456";
    f.backup(OLD,&json!({"model":"claude-sonnet-4-5-20251001","apiKeyHelper":helper,"env":{"ANTHROPIC_API_KEY":secret,"ANTHROPIC_BASE_URL":format!("https://name:password@example.com/anthropic?api_key={secret}#token")}}).to_string());
    f.backup(NEW, &format!("{{invalid {secret}"));
    let rows = list_backup_previews(&f.backups).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].file_name, NEW);
    assert!(!rows[0].valid);
    assert!(rows[0].model.is_none());
    assert!(rows[1].valid);
    assert_eq!(rows[1].model.as_deref(), Some("claude-sonnet-4-5-20251001"));
    assert_eq!(
        rows[1].base_url.as_deref(),
        Some("https://example.com/anthropic")
    );
    assert!(rows[1].helper_present);
    let wire = serde_json::to_string(&rows).unwrap();
    for forbidden in [
        secret,
        "password",
        helper.to_str().unwrap(),
        "apiKeyHelper",
        "ANTHROPIC_API_KEY",
    ] {
        assert!(!wire.contains(forbidden), "preview exposed {forbidden}");
    }
}

#[test]
fn selected_older_backup_restores_exact_values_and_preserves_current_state() {
    let f = Fixture::new();
    f.backup(OLD, "{\"model\":\"older-model\",\"keep\":42}");
    f.backup(NEW, "{\"model\":\"newer-model\"}");
    fs::write(&f.settings, "{\"current\":true}").unwrap();
    let outcome = restore_backup(&f.settings, &f.backups, Some(OLD)).unwrap();
    assert_eq!(outcome.restored_from, OLD);
    assert_eq!(
        read_settings(&f.settings).unwrap(),
        json!({"model":"older-model","keep":42})
    );
    assert_eq!(
        read_settings(&f.backups.join(&outcome.pre_restore_backup)).unwrap(),
        json!({"current":true})
    );
    assert_eq!(
        fs::metadata(f.backups.join(outcome.pre_restore_backup))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
}

#[test]
fn selected_corrupt_backup_refuses_without_creating_a_safety_copy() {
    let f = Fixture::new();
    f.backup(OLD, "{}");
    f.backup(NEW, "[valid JSON but not an object]");
    fs::write(&f.settings, "{\"current\":42}").unwrap();
    assert!(restore_backup(&f.settings, &f.backups, Some(NEW)).is_err());
    assert_eq!(fs::read_to_string(&f.settings).unwrap(), "{\"current\":42}");
    assert_eq!(fs::read_dir(&f.backups).unwrap().count(), 2);
    assert!(
        restore_last_backup(&f.settings, &f.backups).is_err(),
        "legacy wrapper still refuses corrupt newest"
    );
}

#[test]
fn selected_recovery_archives_empty_bytes_separately_and_can_skip_bad_newest() {
    let f = Fixture::new();
    f.backup(OLD, "{\"model\":\"older\"}");
    f.backup(NEW, "");
    fs::write(&f.settings, "").unwrap();
    let outcome = recover_settings_selected(&f.settings, &f.backups, true, Some(OLD)).unwrap();
    assert_eq!(outcome.restored_from, OLD);
    assert!(outcome.pre_restore_backup.starts_with("invalid-settings-"));
    assert!(
        fs::read(f.backups.join(outcome.pre_restore_backup))
            .unwrap()
            .is_empty()
    );
    assert_eq!(read_settings(&f.settings).unwrap()["model"], "older");
    fs::write(&f.settings, "bad").unwrap();
    let automatic = recover_settings_selected(&f.settings, &f.backups, true, None).unwrap();
    assert_eq!(automatic.restored_from, OLD);
}

#[test]
fn traversal_unlisted_and_symlink_backups_are_never_selected_or_previewed() {
    let f = Fixture::new();
    fs::write(&f.settings, "{}").unwrap();
    let outside = f.root.join("outside.json");
    fs::write(&outside, "{\"secret\":\"fixture-only\"}").unwrap();
    symlink(&outside, f.backups.join(NEW)).unwrap();
    f.backup(OLD, "{}");
    for name in ["../outside.json", NEW, "settings-backup-123.json"] {
        assert!(restore_backup(&f.settings, &f.backups, Some(name)).is_err());
    }
    assert_eq!(list_backup_previews(&f.backups).unwrap().len(), 1);
    assert_eq!(
        fs::read_to_string(outside).unwrap(),
        "{\"secret\":\"fixture-only\"}"
    );
    let link = f.root.join("linked-backups");
    symlink(&f.backups, &link).unwrap();
    assert!(list_backup_previews(&link).is_err());
    assert!(restore_backup(&f.settings, &link, Some(OLD)).is_err());
}

#[test]
fn oversized_and_wrong_shape_backups_are_visible_but_invalid() {
    let f = Fixture::new();
    f.backup(OLD, "[]");
    let big = fs::File::create(f.backups.join(NEW)).unwrap();
    big.set_len(SETTINGS_MAX_BYTES + 1).unwrap();
    let previews = list_backup_previews(&f.backups).unwrap();
    assert!(previews.iter().all(|p| !p.valid));
    fs::write(&f.settings, "{}").unwrap();
    assert!(restore_backup(&f.settings, &f.backups, Some(NEW)).is_err());
}

#[test]
fn restore_and_recovery_share_apply_lock_and_do_not_mutate_while_held() {
    let f = Fixture::new();
    f.backup(OLD, "{\"from\":\"backup\"}");
    fs::write(&f.settings, "").unwrap();
    let _guard = ApplyLock::acquire(&f.root).unwrap();
    assert!(restore_backup(&f.settings, &f.backups, Some(OLD)).is_err());
    assert!(recover_settings_selected(&f.settings, &f.backups, true, Some(OLD)).is_err());
    assert!(fs::read(&f.settings).unwrap().is_empty());
    assert_eq!(fs::read_dir(&f.backups).unwrap().count(), 1);
}

#[test]
fn recovery_rejects_valid_missing_or_symlink_current_files_and_ambiguous_clean_choice() {
    let f = Fixture::new();
    f.backup(OLD, "{}");
    assert!(recover_settings_selected(&f.settings, &f.backups, false, None).is_err());
    fs::write(&f.settings, "{}").unwrap();
    assert!(recover_settings_selected(&f.settings, &f.backups, true, Some(OLD)).is_err());
    fs::remove_file(&f.settings).unwrap();
    let target = f.root.join("untouched.json");
    fs::write(&target, "").unwrap();
    symlink(&target, &f.settings).unwrap();
    assert!(recover_settings_selected(&f.settings, &f.backups, false, None).is_err());
    assert!(fs::read(&target).unwrap().is_empty());
    fs::remove_file(&f.settings).unwrap();
    fs::write(&f.settings, "[]").unwrap();
    assert!(recover_settings_selected(&f.settings, &f.backups, false, Some(OLD)).is_err());
    let result = recover_settings_selected(&f.settings, &f.backups, false, None).unwrap();
    assert_eq!(result.restored_from, "clean settings");
    assert_eq!(
        read_settings(&f.settings).unwrap(),
        Value::Object(Default::default())
    );
}

#[test]
fn previews_do_not_repeat_credentials_misplaced_in_model_or_endpoint_path() {
    let f = Fixture::new();
    let key = "nvapi-fixture-secret-123456789"; // gitleaks:allow -- fake redaction fixture
    f.backup(OLD,&json!({"model":key,"env":{"ANTHROPIC_BASE_URL":format!("https://example.com/{key}"),"ANTHROPIC_AUTH_TOKEN":key}}).to_string());
    let previews = list_backup_previews(&f.backups).unwrap();
    assert!(previews[0].valid);
    assert!(previews[0].model.is_none());
    assert!(previews[0].base_url.is_none());
    assert!(!serde_json::to_string(&previews).unwrap().contains(key));
}
