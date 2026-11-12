//! V3-04..V3-10: the loopback proxy as a process that outlives the GUI.
//!
//! V2 kept the proxy inside the Tauri process, so quitting the app broke Claude
//! Code. This module owns a small user-space helper instead:
//!
//! * the helper is **the same binary** started with `--proxy-helper`, so there
//!   is no second artifact to build, sign, ship or trust;
//! * it is detached (`process_group(0)`, no inherited stdio), so the GUI can
//!   quit, crash or be force-quit without taking the proxy down;
//! * its port and token are persisted (`proxy-helper.json` 0600,
//!   `secrets/proxy-helper.secret` 0600), so a restart keeps Claude Code
//!   working without a rewrite of `settings.json`;
//! * login start is an **explicit opt-in** that writes a user LaunchAgent —
//!   nothing is installed behind the owner's back.
//!
//! Every operation verifies identity before signalling: a pid is only killed
//! when the process answering on `/healthz` reports that same pid.

use crate::file_secret_store::FileSecretStore;
use crate::proxy;
use crate::secret_store::SecretStore;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Argument that turns the app binary into the headless helper.
pub const HELPER_ARG: &str = "--proxy-helper";
/// Port the helper should try to keep (set by whoever starts it).
const PREFERRED_PORT_ENV: &str = "CCM_PROXY_PREFERRED_PORT";
const TOKEN_SERVICE: &str = "com.claude-config-manager.proxy-helper";
const TOKEN_ACCOUNT: &str = "proxy-helper";
/// Reverse-DNS label of the optional login agent (user domain, no admin rights).
pub const AGENT_LABEL: &str = "io.github.shahid8142.claude-config-manager.proxy";

pub fn state_path(home: &Path) -> PathBuf {
    home.join(".claude-config-manager")
        .join("proxy-helper.json")
}

/// Last port the helper used, kept across stop/start and independent of the
/// live state file (which is removed the moment the helper stops).
pub fn last_port_path(home: &Path) -> PathBuf {
    home.join(".claude-config-manager")
        .join("proxy-helper.port")
}

fn read_last_port(home: &Path) -> Option<u16> {
    let raw = fs::read_to_string(last_port_path(home)).ok()?;
    let port = raw.trim().parse::<u16>().ok()?;
    if port == 0 { None } else { Some(port) }
}

pub fn write_last_port(home: &Path, port: u16) -> Result<(), String> {
    let path = last_port_path(home);
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    fs::write(&path, format!("{port}\n")).map_err(|e| e.to_string())?;
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())
}

pub fn agent_path(home: &Path) -> PathBuf {
    home.join("Library")
        .join("LaunchAgents")
        .join(format!("{AGENT_LABEL}.plist"))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HelperState {
    pub pid: u32,
    pub port: u16,
}

/// What the UI shows. No token, ever.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HelperStatus {
    /// A verified helper answered on its port with the expected pid.
    pub running: bool,
    /// A state file exists but nothing sane is listening — offer a restart.
    pub stale: bool,
    pub pid: Option<u32>,
    pub port: Option<u16>,
    /// The optional login agent file exists.
    pub login_start: bool,
    /// One sentence the UI can show verbatim.
    pub message: String,
}

pub fn write_state(home: &Path, st: &HelperState) -> Result<(), String> {
    let path = state_path(home);
    let dir = path.parent().ok_or("no parent")?;
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    let _ = fs::remove_file(&tmp);
    let body = serde_json::to_string(st).map_err(|e| e.to_string())?;
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&tmp)
            .map_err(|e| e.to_string())?;
        f.write_all(body.as_bytes()).map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
    }
    if let Ok(meta) = fs::symlink_metadata(&path)
        && meta.file_type().is_symlink()
    {
        let _ = fs::remove_file(&tmp);
        return Err("refusing to replace a symlink where the helper state belongs".into());
    }
    fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

pub fn read_state(home: &Path) -> Option<HelperState> {
    let raw = fs::read_to_string(state_path(home)).ok()?;
    let st: HelperState = serde_json::from_str(&raw).ok()?;
    if st.pid == 0 || st.port == 0 {
        return None;
    }
    Some(st)
}

pub fn clear_state(home: &Path) {
    let _ = fs::remove_file(state_path(home));
}

/// The bearer token Claude Code's helper script uses. Persistent across helper
/// restarts so a restart does not invalidate an applied configuration.
pub fn ensure_token(home: &Path) -> Result<String, String> {
    let store = FileSecretStore::for_home(home);
    if let Ok(Some(existing)) = store.get_secret(TOKEN_SERVICE, TOKEN_ACCOUNT)
        && existing.len() >= 32
        && existing.chars().all(|c| c.is_ascii_hexdigit())
    {
        return Ok(existing);
    }
    let token = proxy::random_token();
    store
        .set_secret(TOKEN_SERVICE, TOKEN_ACCOUNT, &token)
        .map_err(|e| e.to_string())?;
    Ok(token)
}

/// Read the `/healthz` snapshot from `port`. `None` means nothing is listening.
pub fn health(port: u16) -> Option<proxy::ProxyStatus> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).ok()?;
    stream
        .set_read_timeout(Some(Duration::from_millis(1500)))
        .ok()?;
    stream
        .set_write_timeout(Some(Duration::from_millis(1500)))
        .ok()?;
    stream
        .write_all(b"GET /healthz HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        .ok()?;
    let mut raw = String::new();
    stream.read_to_string(&mut raw).ok()?;
    let body = raw.split("\r\n\r\n").nth(1)?;
    serde_json::from_str(body.trim()).ok()
}

/// Status without side effects: never starts anything.
pub fn status(home: &Path) -> HelperStatus {
    let login_start = agent_path(home).exists();
    let state = read_state(home);
    let Some(state) = state else {
        return HelperStatus {
            running: false,
            stale: false,
            pid: None,
            port: None,
            login_start,
            message: "Proxy helper is not running.".into(),
        };
    };
    match health(state.port) {
        Some(h) if h.pid == state.pid => HelperStatus {
            running: true,
            stale: false,
            pid: Some(state.pid),
            port: Some(state.port),
            login_start,
            message: format!("Proxy helper is running on 127.0.0.1:{}.", state.port),
        },
        _ => HelperStatus {
            running: false,
            stale: true,
            pid: None,
            port: None,
            login_start,
            message: "The proxy helper stopped; Claude Code needs it running for routed mode."
                .into(),
        },
    }
}

/// Start the detached helper unless a verified one already runs.
pub fn start(home: &Path) -> Result<HelperState, String> {
    let exe = std::env::current_exe().map_err(|e| format!("cannot locate the app binary: {e}"))?;
    start_with_exe(home, &exe)
}

/// Same, with an explicit binary. Integration tests pass the real binary so the
/// spawned process takes the `--proxy-helper` branch in `main`.
pub fn start_with_exe(home: &Path, exe: &Path) -> Result<HelperState, String> {
    if let Some(st) = read_state(home)
        && let Some(h) = health(st.port)
        && h.pid == st.pid
    {
        return Ok(st);
    }
    // Remember the port Claude Code is already pointed at; the helper prefers
    // it so a restart does not need a settings rewrite.
    let preferred = read_state(home)
        .map(|s| s.port)
        .or_else(|| read_last_port(home));
    clear_state(home);
    if !exe.exists() {
        return Err("cannot locate the app binary".into());
    }
    let mut cmd = std::process::Command::new(exe);
    cmd.arg(HELPER_ARG).env("HOME", home);
    if let Some(port) = preferred {
        cmd.env(PREFERRED_PORT_ENV, port.to_string());
    }
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    {
        use std::os::unix::process::CommandExt;
        // Own process group: the GUI quitting (or a terminal Ctrl-C) cannot
        // deliver a signal meant for the app to the helper.
        cmd.process_group(0);
    }
    let child = cmd
        .spawn()
        .map_err(|e| format!("cannot start the proxy helper: {e}"))?;
    let pid = child.id();
    // The helper writes its own state; give it a moment, then read it back and
    // prove the pid answering on the port is the one we spawned.
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        if let Some(st) = read_state(home)
            && st.pid == pid
            && let Some(h) = health(st.port)
            && h.pid == pid
        {
            return Ok(st);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let _ = std::process::Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .status();
    Err("the proxy helper did not come up; nothing was changed".into())
}

/// Stop the helper. Only a pid that proves it is ours is signalled.
pub fn stop(home: &Path) -> Result<bool, String> {
    let Some(st) = read_state(home) else {
        clear_state(home);
        return Ok(false);
    };
    let ours = health(st.port).map(|h| h.pid == st.pid).unwrap_or(false);
    if !ours {
        // Nothing sane is listening: drop the stale state, signal nothing.
        clear_state(home);
        return Ok(false);
    }
    let status = std::process::Command::new("kill")
        .arg("-TERM")
        .arg(st.pid.to_string())
        .status()
        .map_err(|e| format!("cannot signal the proxy helper: {e}"))?;
    if !status.success() {
        clear_state(home);
        return Err("the proxy helper did not accept SIGTERM".into());
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if health(st.port).is_none() {
            clear_state(home);
            return Ok(true);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err("the proxy helper is still listening after SIGTERM".into())
}

pub fn restart(home: &Path) -> Result<HelperState, String> {
    let _ = stop(home);
    start(home)
}

/// `pid` from `id -u`, so the user-domain launchctl target needs no admin right.
fn uid() -> Result<String, String> {
    let out = std::process::Command::new("id")
        .arg("-u")
        .output()
        .map_err(|e| e.to_string())?;
    let uid = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if uid.is_empty() || !uid.chars().all(|c| c.is_ascii_digit()) {
        return Err("cannot determine the current user id".into());
    }
    Ok(uid)
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub fn plist_body(exe: &Path, home: &Path) -> String {
    let log = home.join(".claude-config-manager").join("proxy-helper.log");
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>{label}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{exe}</string>
    <string>{arg}</string>
  </array>
  <key>EnvironmentVariables</key>
  <dict><key>HOME</key><string>{home}</string></dict>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key>
  <dict><key>SuccessfulExit</key><false/></dict>
  <key>ProcessType</key><string>Background</string>
  <key>Umask</key><integer>63</integer>
  <key>StandardOutPath</key><string>{log}</string>
  <key>StandardErrorPath</key><string>{log}</string>
</dict>
</plist>
"#,
        label = AGENT_LABEL,
        exe = xml_escape(&exe.to_string_lossy()),
        arg = HELPER_ARG,
        home = xml_escape(&home.to_string_lossy()),
        log = xml_escape(&log.to_string_lossy()),
    )
}

/// Install the optional login agent. Explicit owner action only.
pub fn install_login_agent(home: &Path) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let path = agent_path(home);
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    fs::write(&path, plist_body(&exe, home)).map_err(|e| e.to_string())?;
    let uid = uid()?;
    let domain = format!("gui/{uid}");
    // Replace any previous registration so re-enabling is idempotent.
    let _ = std::process::Command::new("launchctl")
        .args(["bootout", &domain])
        .arg(&path)
        .output();
    let out = std::process::Command::new("launchctl")
        .args(["bootstrap", &domain])
        .arg(&path)
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if msg.is_empty() {
            "launchctl refused to load the login agent".into()
        } else {
            msg
        });
    }
    Ok(())
}

pub fn uninstall_login_agent(home: &Path) -> Result<bool, String> {
    let path = agent_path(home);
    if !path.exists() {
        return Ok(false);
    }
    if let Ok(uid) = uid() {
        let domain = format!("gui/{uid}");
        let _ = std::process::Command::new("launchctl")
            .args(["bootout", &domain])
            .arg(&path)
            .output();
    }
    fs::remove_file(&path).map_err(|e| e.to_string())?;
    Ok(true)
}

/// Body of the helper process (`--proxy-helper`). Never returns while healthy.
pub fn run_helper() -> i32 {
    let Ok(home_raw) = std::env::var("HOME") else {
        return 2;
    };
    let home = PathBuf::from(home_raw);
    let token = match ensure_token(&home) {
        Ok(t) => t,
        Err(_) => return 3,
    };
    let preferred = std::env::var(PREFERRED_PORT_ENV)
        .ok()
        .and_then(|v| v.trim().parse::<u16>().ok())
        .or_else(|| read_state(&home).map(|s| s.port))
        .or_else(|| read_last_port(&home));
    let handle = match proxy::start_with(home.clone(), preferred, Some(token)) {
        Ok(h) => h,
        Err(_) => return 4,
    };
    let port = handle.port;
    let _ = write_last_port(&home, port);
    if write_state(
        &home,
        &HelperState {
            pid: std::process::id(),
            port,
        },
    )
    .is_err()
    {
        return 5;
    }
    loop {
        std::thread::sleep(Duration::from_secs(3600));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

    fn temp_home(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ccm-helper-unit-{}-{}-{name}",
            std::process::id(),
            N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn token_is_persistent_and_hex() {
        let home = temp_home("token");
        let a = ensure_token(&home).unwrap();
        let b = ensure_token(&home).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.len(), 64, "256 bits, hex");
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));

        use std::os::unix::fs::PermissionsExt;
        let path = home
            .join(".claude-config-manager")
            .join("secrets")
            .join("proxy-helper.secret");
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert!(mode == 0o600, "token must stay owner-only");
    }

    #[test]
    fn status_without_state_is_honest() {
        let home = temp_home("empty");
        let st = status(&home);
        assert!(!st.running && !st.stale && !st.login_start);
        assert_eq!(st.message, "Proxy helper is not running.");
    }

    #[test]
    fn a_dead_state_file_is_reported_as_stale_not_running() {
        let home = temp_home("stale");
        // A pid that cannot be ours plus a closed port.
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        write_state(&home, &HelperState { pid: 999_999, port }).unwrap();
        let st = status(&home);
        assert!(!st.running, "a closed port is never 'running'");
        assert!(st.stale);
        assert!(!stop(&home).unwrap(), "nothing to signal");
    }

    #[test]
    fn plist_names_the_same_binary_and_helper_argument() {
        let home = temp_home("plist");
        let exe = PathBuf::from(
            "/Applications/Claude Config Manager.app/Contents/MacOS/claude-config-manager",
        );
        let body = plist_body(&exe, &home);
        assert!(body.contains(HELPER_ARG));
        assert!(body.contains(AGENT_LABEL));
        assert!(body.contains("RunAtLoad"));
        // launchd creates the log file itself; Umask 077 keeps it owner-only.
        assert!(body.contains("<key>Umask</key><integer>63</integer>"));
        // An ampersand or angle bracket in a path must not break the XML.
        let odd = plist_body(Path::new("/tmp/a&b<c>"), &home);
        assert!(odd.contains("a&amp;b&lt;c&gt;"));
        assert!(!odd.contains("a&b<c>"));
    }
}
