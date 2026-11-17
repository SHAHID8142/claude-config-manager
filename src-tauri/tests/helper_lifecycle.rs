//! V3-10: lifecycle tests against the **real** binary.
//!
//! `CARGO_BIN_EXE_claude-config-manager` is the app binary; started with
//! `--proxy-helper` it becomes the headless helper. That is the same code path
//! the GUI uses, so these tests exercise the actual detachment, state file,
//! health probe, identity check and shutdown rather than mocks.

use claude_config_manager_lib::helper;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

static N: AtomicU32 = AtomicU32::new(0);

fn temp_home(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ccm-v3-lifecycle-{}-{}-{name}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("home");
    dir
}

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_claude-config-manager"))
}

/// Run `ps` and return (ppid, pgid) for a pid.
fn ps_ids(pid: u32) -> (u32, u32) {
    let out = std::process::Command::new("ps")
        .args(["-o", "ppid=,pgid=", "-p"])
        .arg(pid.to_string())
        .output()
        .expect("ps");
    let text = String::from_utf8_lossy(&out.stdout);
    let mut it = text.split_whitespace();
    let ppid = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    let pgid = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    (ppid, pgid)
}

#[test]
fn helper_starts_detached_serves_healthz_and_stops() {
    let home = temp_home("basic");
    let started = helper::start_with_exe(&home, &bin()).expect("helper starts");

    assert!(started.pid > 0);
    let st = helper::status(&home);
    assert!(st.running, "status must see the helper: {}", st.message);
    assert_eq!(st.port, Some(started.port));
    assert_eq!(st.pid, Some(started.pid));
    assert!(!st.stale);

    // A real /healthz answers with proxy data — no token, no body.
    let h = helper::health(started.port).expect("healthz");
    assert_eq!(
        h.pid, started.pid,
        "the pid answering must be the one we started"
    );
    assert!(h.running);

    // Detached: its own process group, so a signal to the app's group (quit,
    // Ctrl-C in a terminal) cannot reach it. Its parent is this test process
    // now, and becomes launchd the moment this process exits.
    let (ppid, pgid) = ps_ids(started.pid);
    assert_eq!(
        pgid, started.pid,
        "helper must own its process group (ppid {ppid})"
    );

    assert!(
        helper::stop(&home).expect("stop"),
        "stop must report success"
    );
    assert!(!helper::status(&home).running, "nothing listens afterwards");
    assert!(
        helper::health(started.port).is_none(),
        "the port must be released"
    );
    assert!(helper::read_state(&home).is_none(), "state file removed");
}

#[test]
fn token_survives_and_is_reused_on_restart() {
    let home = temp_home("token-reuse");
    let token = helper::ensure_token(&home).expect("token");
    let first = helper::start_with_exe(&home, &bin()).expect("start");
    helper::stop(&home).expect("stop");

    let again = helper::ensure_token(&home).expect("token again");
    assert_eq!(
        token, again,
        "restart must not invalidate Claude Code's token"
    );

    // The third start reuses the persisted port when it is free, so an applied
    // configuration keeps pointing at a live proxy.
    let second = helper::start_with_exe(&home, &bin()).expect("restart");
    assert_eq!(
        second.port, first.port,
        "preferred port is reused on restart"
    );
    helper::stop(&home).expect("stop");
}

#[test]
fn double_start_returns_the_same_helper_instead_of_a_second_one() {
    let home = temp_home("idempotent");
    let a = helper::start_with_exe(&home, &bin()).expect("start");
    let b = helper::start_with_exe(&home, &bin()).expect("second start");
    assert_eq!(a.pid, b.pid, "start is idempotent");
    assert_eq!(a.port, b.port);
    helper::stop(&home).expect("stop");
}

#[test]
fn a_killed_helper_is_reported_stale_and_restart_recovers() {
    let home = temp_home("crash");
    let started = helper::start_with_exe(&home, &bin()).expect("start");

    // Simulate a crash (SIGKILL, no cleanup of the state file).
    let _ = std::process::Command::new("kill")
        .arg("-KILL")
        .arg(started.pid.to_string())
        .status();
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while std::time::Instant::now() < deadline && helper::health(started.port).is_some() {
        std::thread::sleep(Duration::from_millis(50));
    }

    let st = helper::status(&home);
    assert!(!st.running, "a dead helper is never reported running");
    assert!(st.stale, "the leftover state file must be reported stale");

    // Restart recovers without the owner clearing anything by hand.
    let again = helper::start_with_exe(&home, &bin()).expect("restart after crash");
    assert!(helper::status(&home).running);
    assert_eq!(again.port, started.port, "same port after recovery");
    helper::stop(&home).expect("stop");
}

#[test]
fn stop_never_signals_a_pid_it_cannot_verify() {
    let home = temp_home("no-signal");
    // A state file pointing at a pid that is not answering on that port: the
    // helper must clear the file and send no signal at all.
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);
    helper::write_state(
        &home,
        &helper::HelperState {
            pid: std::process::id(), // a live pid, but not a helper
            port,
        },
    )
    .expect("state");

    assert!(
        !helper::stop(&home).expect("stop"),
        "nothing verified to stop"
    );
    assert!(helper::read_state(&home).is_none());
}

#[test]
fn a_second_raw_helper_cannot_rebind_or_overwrite_the_active_state() {
    let home = temp_home("raw-duplicate");
    let first = helper::start_with_exe(&home, &bin()).unwrap();
    let mut duplicate = std::process::Command::new(bin())
        .arg(helper::HELPER_ARG)
        .env("HOME", &home)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let exit = loop {
        if let Some(exit) = duplicate.try_wait().unwrap() {
            break exit;
        }
        if std::time::Instant::now() > deadline {
            let _ = duplicate.kill();
            helper::stop(&home).unwrap();
            panic!("duplicate helper did not exit");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(exit.code(), Some(6));
    let state = helper::read_state(&home).unwrap();
    assert_eq!(state.pid, first.pid);
    assert_eq!(state.port, first.port);
    assert_eq!(helper::health(first.port).unwrap().pid, first.pid);
    helper::stop(&home).unwrap();
    let restarted = helper::start_with_exe(&home, &bin()).unwrap();
    assert_eq!(restarted.port, first.port);
    helper::stop(&home).unwrap();
}

#[test]
#[ignore = "requires CCM_OLD_HELPER_EXE pointing to a verified older helper; fixture home only"]
fn older_helper_is_upgraded_before_new_policy_is_saved() {
    let home = temp_home("version-upgrade");
    let old = PathBuf::from(std::env::var_os("CCM_OLD_HELPER_EXE").expect("older binary fixture"));
    let before = helper::start_with_exe(&home, &old).unwrap();
    let old_version = helper::health(before.port).unwrap().version;
    assert_ne!(old_version.as_deref(), Some(env!("CARGO_PKG_VERSION")));
    let token = helper::ensure_token(&home).unwrap();
    let after = helper::start_with_exe(&home, &bin()).unwrap();
    assert_ne!(before.pid, after.pid);
    assert_eq!(before.port, after.port);
    assert_eq!(
        helper::health(after.port).unwrap().version.as_deref(),
        Some(env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(helper::ensure_token(&home).unwrap(), token);
    helper::stop(&home).unwrap();
    std::fs::remove_dir_all(home).unwrap();
}
