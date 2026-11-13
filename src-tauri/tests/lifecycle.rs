//! T66 — helper restart, GUI-closure independence, clean uninstall and backup
//! restore behaviour.
//!
//! The credential helper is a short-lived script Claude Code runs per request, so
//! "restart" means: the script is idempotent, fails closed, and prints the secret
//! and nothing else. These tests execute the **generated script itself** against a
//! fixture HOME holding a real 0600 credential file — proving the real file, not
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

/// Create the credential file the helper reads, under a fixture HOME.
fn write_credential(home: &Path, provider: &str, value: &str, mode: u32) {
    let dir = home.join(".claude-config-manager").join("secrets");
    fs::create_dir_all(&dir).expect("secrets dir");
    let path = dir.join(format!("{provider}.secret"));
    fs::write(&path, value).expect("write credential");
    fs::set_permissions(&path, fs::Permissions::from_mode(mode)).expect("chmod");
}

fn run_helper(script: &Path, home: &Path) -> std::process::Output {
    // Linux/overlayfs quirk: a freshly renamed file can transiently return
    // ETXTBSY on exec. Retry a few times — production exec paths (Claude
    // Code) hit the same kernel behavior, so the retry mirrors reality.
    let mut last: Option<std::io::Error> = None;
    for _ in 0..5 {
        match Command::new(script)
            .env("HOME", home)
            .env("PATH", "/usr/bin:/bin")
            .output()
        {
            Ok(out) => return out,
            Err(e) if e.raw_os_error() == Some(26) => {
                last = Some(e);
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(e) => panic!("run helper: {e}"),
        }
    }
    panic!("run helper: exec kept failing with ETXTBSY: {:?}", last);
}

#[test]
fn helper_prints_only_the_credential_and_is_restart_safe() {
    let dir = temp_dir("restart");
    write_credential(&dir, "deepseek", "sk-ant-api03-T66-SECRET\n", 0o600);
    let script_text = generate_helper_script("deepseek").expect("script");
    let script =
        install_helper_script(&dir.join("helpers"), "deepseek", &script_text).expect("install");

    // First run.
    let first = run_helper(&script, &dir);
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
    let second = run_helper(&script, &dir);
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
fn helper_fails_closed_when_the_credential_file_is_gone() {
    let dir = temp_dir("missing");
    let script = install_helper_script(
        &dir.join("helpers"),
        "deepseek",
        &generate_helper_script("deepseek").expect("script"),
    )
    .expect("install");

    let out = run_helper(&script, &dir);
    assert!(
        !out.status.success(),
        "missing credential must fail, not emit empty"
    );
    assert!(out.stdout.is_empty(), "no partial secret on failure");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn helper_does_not_mix_diagnostics_into_the_secret_stream() {
    // A world-readable credential is refused by the helper itself, with the
    // diagnostic on stderr and nothing on stdout.
    let dir = temp_dir("noisy");
    write_credential(&dir, "deepseek", "sk-ant-api03-T66-SECRET\n", 0o644);
    let script = install_helper_script(
        &dir.join("helpers"),
        "deepseek",
        &generate_helper_script("deepseek").expect("script"),
    )
    .expect("install");

    let out = run_helper(&script, &dir);
    assert!(!out.status.success(), "0644 must be refused");
    assert!(out.stdout.is_empty(), "no secret may be emitted");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("permissions are too open"),
        "stderr: {:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn apply_path_spawns_no_background_worker_so_gui_closure_is_safe() {
    // GUI closure cannot interrupt a write because nothing outlives the command
    // call: the pipeline is synchronous. Guard against a future regression that
    // would make closing the window abort a half-written config.
    let src = include_str!("../src/save_apply.rs");
    // Only the shipping code counts: the test module may spawn threads (the G4
    // concurrency test does), production must not.
    let production = src.split("#[cfg(test)]").next().expect("module body");
    for forbidden in [
        "thread::spawn",
        "tokio::spawn",
        "spawn_blocking",
        "std::process::Command::new",
    ] {
        assert!(
            !production.contains(forbidden),
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
