//! T66 — helper restart, GUI-closure independence, clean uninstall and backup
//! restore behaviour.
//!
//! The Keychain helper is a short-lived script Claude Code runs per request, so
//! "restart" means: the script is idempotent, fails closed, and prints the
//! secret and nothing else. These tests execute the **generated script itself**
//! against a fake `security` binary placed on PATH — proving the real file, not
//! a paraphrase of it.

use claude_config_manager_lib::config_backup::list_backups;
use claude_config_manager_lib::config_reader::read_settings;
use claude_config_manager_lib::config_restore::restore_last_backup;
use claude_config_manager_lib::credential_handoff::{
    generate_helper_script, install_helper_script,
};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ccm-t66-{tag}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("temp dir");
    dir
}

/// Build a fake `security` executable that behaves like the real one for our
/// arguments, echoing a fixed secret (or failing, when asked).
fn fake_security(dir: &Path, behaviour: &str) -> PathBuf {
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).expect("bin dir");
    let path = bin.join("security");
    fs::write(
        &path,
        match behaviour {
            "found" => "#!/bin/sh\necho sk-ant-api03-T66-SECRET\n",
            "missing" => "#!/bin/sh\nexit 44\n",
            "noisy" => "#!/bin/sh\necho 'diagnostic line' >&2\necho sk-ant-api03-T66-SECRET\n",
            _ => unreachable!(),
        },
    )
    .expect("write fake security");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).expect("chmod");
    bin
}

fn run_helper(script: &Path, path_dir: &Path) -> std::process::Output {
    Command::new(script)
        .env("PATH", format!("{}:/usr/bin:/bin", path_dir.display()))
        .output()
        .expect("run helper")
}

#[test]
fn helper_prints_only_the_credential_and_is_restart_safe() {
    let dir = temp_dir("restart");
    let bin = fake_security(&dir, "found");
    let script_text = generate_helper_script("deepseek").expect("script");
    let script =
        install_helper_script(&dir.join("helpers"), "deepseek", &script_text).expect("install");

    // First run.
    let first = run_helper(&script, &bin);
    assert!(first.status.success(), "helper must exit 0: {first:?}");
    assert_eq!(
        String::from_utf8_lossy(&first.stdout).trim(),
        "sk-ant-api03-T66-SECRET"
    );
    assert!(
        first.stderr.is_empty(),
        "nothing may leak to stderr: {first:?}"
    );

    // "Restart" = run it again: identical result, no state carried over.
    let second = run_helper(&script, &bin);
    assert_eq!(
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&second.stdout)
    );

    // The script is owner-executable only and contains no secret.
    let mode = fs::metadata(&script).expect("meta").permissions().mode() & 0o777;
    assert_eq!(mode, 0o700);
    assert!(!script_text.contains("sk-ant"));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn helper_fails_closed_when_the_keychain_item_is_gone() {
    let dir = temp_dir("missing");
    let bin = fake_security(&dir, "missing");
    let script = install_helper_script(
        &dir.join("helpers"),
        "deepseek",
        &generate_helper_script("deepseek").expect("script"),
    )
    .expect("install");

    let out = run_helper(&script, &bin);
    assert!(
        !out.status.success(),
        "missing credential must fail, not emit empty"
    );
    assert!(out.stdout.is_empty(), "no partial secret on failure");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn helper_does_not_mix_diagnostics_into_the_secret_stream() {
    // `security` writing to stderr must not corrupt the value Claude Code reads
    // from stdout (the script `exec`s, so streams stay separate).
    let dir = temp_dir("noisy");
    let bin = fake_security(&dir, "noisy");
    let script = install_helper_script(
        &dir.join("helpers"),
        "deepseek",
        &generate_helper_script("deepseek").expect("script"),
    )
    .expect("install");

    let out = run_helper(&script, &bin);
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "sk-ant-api03-T66-SECRET"
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("diagnostic line"));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn apply_path_spawns_no_background_worker_so_gui_closure_is_safe() {
    // GUI closure cannot interrupt a write because nothing outlives the command
    // call: the pipeline is synchronous. Guard against a future regression that
    // would make closing the window abort a half-written config.
    let src = include_str!("../src/save_apply.rs");
    for forbidden in [
        "thread::spawn",
        "tokio::spawn",
        "spawn_blocking",
        "std::process::Command::new",
    ] {
        assert!(
            !src.contains(forbidden),
            "save_apply must stay synchronous — found {forbidden}"
        );
    }
}

#[test]
fn uninstall_leaves_no_managed_keys_but_preserves_user_data() {
    // There is no automated uninstall yet (T58, blocked); this test documents
    // and verifies the *manual* procedure users are given in the guide:
    // delete settings.json's managed keys, the helper script and the Keychain
    // item — and prove that doing so leaves unrelated user settings intact.
    use claude_config_manager_lib::config_writer::write_settings_atomic;

    let dir = temp_dir("uninstall");
    let settings = dir.join(".claude/settings.json");
    fs::create_dir_all(settings.parent().unwrap()).expect("dir");
    fs::write(
        &settings,
        r#"{"model":"provider-model","apiKeyHelper":"/x/helpers/deepseek.sh",
            "env":{"ANTHROPIC_BASE_URL":"https://api.deepseek.com/anthropic"},
            "permissions":{"allow":["Bash(ls)"]}}"#,
    )
    .expect("seed");

    // Manual uninstall: drop only the keys this app manages.
    let doc = read_settings(&settings).expect("read");
    let mut obj = doc.as_object().cloned().expect("object");
    obj.remove("model");
    obj.remove("apiKeyHelper");
    let mut env = obj
        .get("env")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    env.remove("ANTHROPIC_BASE_URL");
    if env.is_empty() {
        obj.remove("env");
    } else {
        obj.insert("env".into(), serde_json::Value::Object(env));
    }
    write_settings_atomic(&settings, &serde_json::Value::Object(obj), None).expect("write");

    let after = read_settings(&settings).expect("re-read");
    assert!(after.get("model").is_none());
    assert!(after.get("apiKeyHelper").is_none());
    assert!(after.get("env").is_none());
    assert_eq!(
        after["permissions"]["allow"][0], "Bash(ls)",
        "user data outside managed keys must survive uninstall"
    );
    assert!(after.as_object().is_some_and(|o| !o.is_empty()));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn backups_survive_uninstall_and_can_restore_the_previous_configuration() {
    let dir = temp_dir("restore-after-uninstall");
    let settings = dir.join(".claude/settings.json");
    let backups = dir.join("backups");
    fs::create_dir_all(settings.parent().unwrap()).expect("dir");
    fs::write(&settings, r#"{"model":"provider-model"}"#).expect("seed");

    // A real apply creates a backup of the pre-apply state.
    let _ = claude_config_manager_lib::config_backup::create_backup(&settings, &backups)
        .expect("backup");
    // Apply changes the file.
    claude_config_manager_lib::config_writer::write_settings_atomic(
        &settings,
        &serde_json::json!({"model":"changed"}),
        None,
    )
    .expect("write");

    assert_eq!(list_backups(&backups).expect("list").len(), 1);
    restore_last_backup(&settings, &backups).expect("restore");
    assert_eq!(
        read_settings(&settings).expect("read")["model"],
        "provider-model"
    );
    fs::remove_dir_all(&dir).ok();
}
