use claude_config_manager_lib::{
    config_backup, config_reader, config_restore,
    credential_vault::CredentialVault,
    file_secret_store::FileSecretStore,
    save_apply::{RoleOverrides, SaveApplyService},
    secret_store::SecretStore,
};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "ccm-repair-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(path.join(".claude")).unwrap();
        Self(path)
    }
    fn service(&self) -> SaveApplyService {
        let mut service =
            SaveApplyService::for_production(&self.0, Arc::new(FileSecretStore::for_home(&self.0)));
        service.config_dir_override = None;
        service
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn legacy_alias_applies_and_helper_reads_the_original_secret_file() {
    let home = Fixture::new();
    let store = FileSecretStore::for_home(&home.0);
    store
        .set_secret(
            "com.claude-config-manager.openrouter",
            "openrouter",
            "fixture-only",
        )
        .unwrap();
    let service = home.service();
    let out = service
        .apply_with_window(
            "openrouter",
            Some("model-a"),
            Some("https://openrouter.ai/api"),
            None,
            &RoleOverrides::default(),
            None,
            false,
            Some("openrouter-legacy"),
        )
        .unwrap();
    let script = std::fs::read_to_string(out.helper_path).unwrap();
    assert!(script.contains("secrets/openrouter.secret"));
    assert!(!script.contains("secrets/openrouter-legacy.secret"));
}

#[test]
fn invalid_settings_cannot_replace_a_working_secret_or_helper() {
    let home = Fixture::new();
    let service = home.service();
    service
        .apply(
            "openrouter",
            Some("model-a"),
            Some("https://openrouter.ai/api"),
            Some("original-fixture"),
        )
        .unwrap();
    let helper = service.helpers_dir.join("openrouter.sh");
    let previous = std::fs::read(&helper).unwrap();
    std::fs::write(home.0.join(".claude/settings.json"), "").unwrap();
    assert!(
        service
            .apply(
                "openrouter",
                None,
                Some("https://openrouter.ai/api"),
                Some("replacement-fixture")
            )
            .is_err()
    );
    let store = FileSecretStore::for_home(&home.0);
    assert_eq!(
        store
            .get_secret("com.claude-config-manager.openrouter", "openrouter")
            .unwrap()
            .as_deref(),
        Some("original-fixture")
    );
    assert_eq!(std::fs::read(helper).unwrap(), previous);
}

#[test]
fn backup_failure_rolls_back_key_replacement() {
    let home = Fixture::new();
    let mut service = home.service();
    service
        .apply(
            "openrouter",
            Some("model-a"),
            Some("https://openrouter.ai/api"),
            Some("original-fixture"),
        )
        .unwrap();
    let bad = home.0.join("not-a-directory");
    std::fs::write(&bad, "blocked").unwrap();
    service.backups_dir = bad;
    assert!(
        service
            .apply(
                "openrouter",
                None,
                Some("https://openrouter.ai/api"),
                Some("replacement-fixture")
            )
            .is_err()
    );
    let store = FileSecretStore::for_home(&home.0);
    assert_eq!(
        store
            .get_secret("com.claude-config-manager.openrouter", "openrouter")
            .unwrap()
            .as_deref(),
        Some("original-fixture")
    );
}

#[test]
fn recovery_skips_bad_backup_and_archives_empty_bytes_outside_restore_history() {
    let home = Fixture::new();
    let settings = home.0.join(".claude/settings.json");
    let backups = config_backup::default_backup_dir(&home.0);
    std::fs::create_dir_all(&backups).unwrap();
    std::fs::write(
        backups.join("settings-backup-1.json"),
        "{\"model\":\"saved\"}",
    )
    .unwrap();
    std::fs::write(backups.join("settings-backup-2.json"), "{bad").unwrap();
    std::fs::write(&settings, "").unwrap();
    let out = config_restore::recover_settings(&settings, &backups, true).unwrap();
    assert_eq!(out.restored_from, "settings-backup-1.json");
    assert!(out.pre_restore_backup.starts_with("invalid-settings-"));
    assert!(
        std::fs::read(backups.join(out.pre_restore_backup))
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        config_reader::read_settings(&settings).unwrap()["model"],
        "saved"
    );
}

#[test]
fn indexed_and_internal_secret_files_never_become_phantom_providers() {
    let home = Fixture::new();
    let vault = CredentialVault::for_home(&home.0);
    vault.add("openrouter", "first", "fixture-only").unwrap();
    let store = FileSecretStore::for_home(&home.0);
    store
        .set_secret(
            "com.claude-config-manager.proxy-helper",
            "proxy-helper",
            "fixture-only",
        )
        .unwrap();
    assert_eq!(vault.load().unwrap().credentials.len(), 1);
    assert!(
        vault
            .resolve("deepseek", Some("openrouter-1"))
            .unwrap()
            .is_none()
    );
    vault.set_enabled("openrouter-1", false).unwrap();
    assert!(!vault.has_any("openrouter").unwrap());
    assert!(vault.resolve("openrouter", None).unwrap().is_none());
}

#[test]
fn concurrent_first_run_file_is_preserved() {
    let home = Fixture::new();
    let settings = home.0.join(".claude/settings.json");
    std::fs::write(&settings, "{\"external\":true}").unwrap();
    let result = claude_config_manager_lib::config_writer::write_settings_atomic_checked(
        &settings,
        &serde_json::json!({"model":"new"}),
        None,
    );
    assert!(matches!(
        result,
        Err(config_reader::ConfigError::ConcurrentModification)
    ));
    assert_eq!(
        config_reader::read_settings(&settings).unwrap()["external"],
        true
    );
}

#[test]
fn explicit_role_reset_preserves_other_roles_and_unmanaged_values() {
    let current = serde_json::json!({"env":{"ANTHROPIC_DEFAULT_HAIKU_MODEL":"old", "ANTHROPIC_DEFAULT_SONNET_MODEL":"keep", "USER_VALUE":"untouched"}});
    let desired = claude_config_manager_lib::config_merge::DesiredSettings {
        clear_env_roles: [true, false, false],
        ..Default::default()
    };
    let merged =
        claude_config_manager_lib::config_merge::apply_desired(&current, &desired).unwrap();
    assert!(merged["env"].get("ANTHROPIC_DEFAULT_HAIKU_MODEL").is_none());
    assert_eq!(merged["env"]["ANTHROPIC_DEFAULT_SONNET_MODEL"], "keep");
    assert_eq!(merged["env"]["USER_VALUE"], "untouched");
}

#[test]
fn dated_model_ids_are_not_mistaken_for_api_credentials() {
    for id in [
        "claude-sonnet-4-5-20250929",
        "gemini-3.0-pro-001",
        "provider/model-20261117",
    ] {
        assert!(
            claude_config_manager_lib::config_merge::validate_model_id(id).is_ok(),
            "{id}"
        );
    }
    assert!(
        claude_config_manager_lib::config_merge::validate_model_id("sk-or-fixture-not-a-model")
            .is_err()
    );
}

#[test]
fn metadata_writes_do_not_follow_predictable_temp_symlinks() {
    use claude_config_manager_lib::{credential_index::CredentialIndex, routing_config};
    use std::os::unix::fs::symlink;
    let home = Fixture::new();
    let root = home.0.join(".claude-config-manager");
    std::fs::create_dir_all(&root).unwrap();
    let outside = home.0.join("untouched.txt");
    std::fs::write(&outside, "original").unwrap();
    symlink(&outside, root.join("credentials.json.tmp")).unwrap();
    symlink(&outside, root.join("routing.json.tmp")).unwrap();
    CredentialVault::for_home(&home.0)
        .save(&CredentialIndex::default())
        .unwrap();
    routing_config::save(&home.0, &routing_config::RoutingConfig::default()).unwrap();
    assert_eq!(std::fs::read_to_string(outside).unwrap(), "original");
}

#[test]
fn first_run_settings_can_create_the_missing_directory() {
    use claude_config_manager_lib::config_writer::write_settings_atomic_checked;
    let home = Fixture::new();
    let path = home.0.join("new-config/settings.json");
    write_settings_atomic_checked(&path, &serde_json::json!({"model":"fixture-model"}), None)
        .unwrap();
    assert_eq!(
        config_reader::read_settings(&path).unwrap()["model"],
        "fixture-model"
    );
}

#[test]
fn nvidia_save_writes_local_helper_settings_and_keeps_fallbacks() {
    use claude_config_manager_lib::{
        nvidia_setup,
        routing_config::{self, RouteTarget, RoutingConfig},
    };
    let home = Fixture::new();
    let svc = home.service();
    routing_config::save(
        &home.0,
        &RoutingConfig {
            version: 1,
            enabled: false,
            targets: vec![RouteTarget {
                id: "backup".into(),
                provider_id: "openrouter".into(),
                credential_id: None,
                model: Some("backup-model".into()),
                base_url: "https://openrouter.ai/api".into(),
                enabled: true,
            }],
        },
    )
    .unwrap();
    let roles = RoleOverrides {
        sonnet: Some("nvidia/nemotron-3-ultra-550b-a55b".into()),
        ..Default::default()
    };
    let outcome = nvidia_setup::apply(
        &svc,
        "nvidia/nemotron-3-ultra-550b-a55b",
        Some("fixture-nvidia-key"),
        &roles,
        Some(1_000_000),
        false,
        || Ok(54321),
    )
    .unwrap();
    let settings = config_reader::read_settings(&home.0.join(".claude/settings.json")).unwrap();
    assert_eq!(
        settings["env"]["ANTHROPIC_BASE_URL"],
        "http://127.0.0.1:54321"
    );
    assert_eq!(
        settings["env"]["ANTHROPIC_DEFAULT_SONNET_MODEL"],
        "nvidia/nemotron-3-ultra-550b-a55b"
    );
    assert!(!settings.to_string().contains("fixture-nvidia-key"));
    assert!(outcome.helper_path.ends_with("proxy.sh"));
    let cfg = routing_config::load(&home.0).unwrap();
    assert_eq!(cfg.targets[0].provider_id, "nvidia-nim");
    assert!(cfg.targets[0].credential_id.is_none());
    assert_eq!(cfg.targets[1].id, "backup");
    assert!(cfg.enabled);
}
#[test]
fn nvidia_preflight_and_failed_helper_activation_preserve_settings_and_policy() {
    use claude_config_manager_lib::{nvidia_setup, routing_config};
    let home = Fixture::new();
    let svc = home.service();
    let path = home.0.join(".claude/settings.json");
    std::fs::write(&path, "").unwrap();
    assert!(
        nvidia_setup::apply(
            &svc,
            "model",
            Some("fixture"),
            &RoleOverrides::default(),
            None,
            false,
            || panic!("must validate before starting helper")
        )
        .is_err()
    );
    assert!(
        CredentialVault::for_home(&home.0)
            .list("nvidia-nim")
            .unwrap()
            .is_empty()
    );
    std::fs::write(&path, "{\"unrelated\":42}").unwrap();
    let before = routing_config::load(&home.0).unwrap();
    assert!(
        nvidia_setup::apply(
            &svc,
            "model",
            Some("fixture"),
            &RoleOverrides::default(),
            None,
            false,
            || Err("fixture helper unavailable".into())
        )
        .is_err()
    );
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "{\"unrelated\":42}"
    );
    assert_eq!(routing_config::load(&home.0).unwrap(), before);
}

#[test]
fn duplicate_keys_reuse_metadata_and_unknown_remove_keeps_internal_token() {
    let home = Fixture::new();
    let vault = CredentialVault::for_home(&home.0);
    let first = vault
        .add("nvidia-nim", "First", "fixture-identical")
        .unwrap();
    let second = vault
        .add("nvidia-nim", "Again", "fixture-identical")
        .unwrap();
    assert_eq!(first.id, second.id);
    assert_eq!(vault.list("nvidia-nim").unwrap().len(), 1);
    let store = FileSecretStore::for_home(&home.0);
    store
        .set_secret(
            "com.claude-config-manager.proxy-helper",
            "proxy-helper",
            "fixture-internal",
        )
        .unwrap();
    assert!(!vault.remove("proxy-helper").unwrap());
    assert_eq!(
        store
            .get_secret("com.claude-config-manager.proxy-helper", "proxy-helper")
            .unwrap()
            .as_deref(),
        Some("fixture-internal")
    );
}

#[test]
fn failed_nvidia_activation_removes_staged_key_and_retains_concurrent_settings_edit() {
    use claude_config_manager_lib::{nvidia_setup, transaction_journal};
    let home = Fixture::new();
    let settings = home.0.join(".claude/settings.json");
    std::fs::write(&settings, "{}").unwrap();
    let err = nvidia_setup::apply(
        &home.service(),
        "fixture-model",
        Some("fixture-new-key"),
        &RoleOverrides::default(),
        None,
        false,
        || {
            std::fs::write(&settings, "{\"externalEditor\":true}").unwrap();
            Err("fixture activation failed".into())
        },
    );
    assert!(err.is_err());
    assert!(
        CredentialVault::for_home(&home.0)
            .list("nvidia-nim")
            .unwrap()
            .is_empty()
    );
    assert!(
        std::fs::read_dir(home.0.join(".claude-config-manager/secrets"))
            .unwrap()
            .next()
            .is_none()
    );
    assert_eq!(
        std::fs::read_to_string(settings).unwrap(),
        "{\"externalEditor\":true}"
    );
    assert!(transaction_journal::pending(&home.0).unwrap().is_none());
}

#[test]
fn helper_commands_quote_spaces_quotes_and_shell_metacharacters_as_one_path() {
    use claude_config_manager_lib::credential_handoff::{helper_command, helper_command_path};
    for raw in [
        "/fixture space/helpers/nvidia.sh",
        "/fixture'quote/helpers/nvidia.sh",
        "/fixture$(touch BAD)/helpers/nvidia.sh",
    ] {
        let path = std::path::Path::new(raw);
        let command = helper_command(path);
        assert_eq!(helper_command_path(&command).as_deref(), Some(path));
        let output = std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg(format!("printf '%s' {command}"))
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(String::from_utf8(output.stdout).unwrap(), raw);
    }
    for command in [
        "/tmp/a.sh; echo BAD",
        "'/tmp/a.sh'; echo BAD",
        "/tmp/a.sh $(bad)",
    ] {
        assert!(helper_command_path(command).is_none());
    }
}

#[test]
fn failed_switching_activation_restores_saved_policy_and_token() {
    use claude_config_manager_lib::{
        routing_config::{self, RouteTarget, RoutingConfig},
        transaction_journal,
    };
    let home = Fixture::new();
    CredentialVault::for_home(&home.0)
        .add("openrouter", "Work", "fixture-work")
        .unwrap();
    let before = routing_config::load(&home.0).unwrap();
    let cfg = RoutingConfig {
        version: 1,
        enabled: true,
        targets: vec![RouteTarget {
            id: "work".into(),
            provider_id: "openrouter".into(),
            credential_id: None,
            model: Some("fixture-model".into()),
            base_url: "https://openrouter.ai/api".into(),
            enabled: true,
        }],
    };
    assert!(
        home.service()
            .apply_routing_policy(&cfg, None, || Err("fixture start failed".into()))
            .is_err()
    );
    assert_eq!(routing_config::load(&home.0).unwrap(), before);
    assert!(
        !home
            .0
            .join(".claude-config-manager/secrets/proxy-helper.secret")
            .exists()
    );
    assert!(transaction_journal::pending(&home.0).unwrap().is_none());
}
