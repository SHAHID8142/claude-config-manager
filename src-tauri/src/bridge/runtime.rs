//! Bridge runtime (T51/T52/T53/T54): loopback-only config generation and the
//! detached `ccr serve` lifecycle.
//!
//! - Config generation never inlines key material: providers reference the
//!   credential **file** the credential store maintains (0600), and the bridge
//!   reads it at request time.
//! - The gateway binds 127.0.0.1 only — the generated config says so, and the
//!   writer refuses any other host.
//! - `start` spawns argv-only (`node cli.js serve --host 127.0.0.1 …`) in a
//!   detached process group so the bridge survives GUI closure (T53); a pid
//!   file in the state dir backs status/stop across app restarts.

use super::manifest::{GATEWAY_PORT, PINNED};
use serde::Serialize;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const BRIDGE_HTTP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

#[derive(Debug, Clone, PartialEq)]
pub enum BridgeRuntimeError {
    NotInstalled,
    NotConsented,
    Config(String),
    Io(String),
}

impl std::fmt::Display for BridgeRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BridgeRuntimeError::NotInstalled => {
                write!(f, "the bridge is not installed yet")
            }
            BridgeRuntimeError::NotConsented => {
                write!(f, "bridge needs your explicit consent first")
            }
            BridgeRuntimeError::Config(m) => write!(f, "bridge config rejected ({m})"),
            BridgeRuntimeError::Io(m) => write!(f, "bridge runtime error ({m})"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeStatus {
    pub running: bool,
    pub pid: Option<u32>,
    pub gateway_port: u16,
    pub version: String,
}

pub fn pid_file(state_dir: &Path) -> PathBuf {
    state_dir.join("bridge-serve.pid")
}

/// The generated gateway config. Serialized to `gateway.config.json` in the
/// state dir. Only 127.0.0.1 is ever written as host.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GatewayProviderConfig {
    pub name: String,
    pub base_url: String,
    /// Path to the 0600 credential file — the bridge reads it per request.
    pub api_key_file: String,
}

/// Generate the gateway config JSON. `host` is refused unless loopback (T51).
pub fn generate_gateway_config(
    provider: &GatewayProviderConfig,
    port: u16,
) -> Result<String, BridgeRuntimeError> {
    if !provider.base_url.starts_with("https://")
        && !provider.base_url.starts_with("http://127.0.0.1")
        && !provider.base_url.starts_with("http://localhost")
    {
        return Err(BridgeRuntimeError::Config(
            "provider base URL must be https (or a loopback test endpoint)".into(),
        ));
    }
    if provider.name.trim().is_empty() || provider.name.len() > 64 {
        return Err(BridgeRuntimeError::Config("provider name invalid".into()));
    }
    let doc = serde_json::json!({
        "host": "127.0.0.1",
        "port": port,
        "providers": [
            {
                "name": provider.name,
                "api_base_url": provider.base_url,
                "api_key_file": provider.api_key_file,
                "models": []
            }
        ]
    });
    serde_json::to_string_pretty(&doc).map_err(|e| BridgeRuntimeError::Config(e.to_string()))
}

/// Write the generated config + assert the loopback invariant on read-back.
pub fn write_gateway_config(
    state_dir: &Path,
    provider: &GatewayProviderConfig,
    port: u16,
) -> Result<PathBuf, BridgeRuntimeError> {
    let json = generate_gateway_config(provider, port)?;
    std::fs::create_dir_all(state_dir).map_err(|e| BridgeRuntimeError::Io(e.to_string()))?;
    let path = state_dir.join("gateway.config.json");
    // Read-back verification (F07 discipline applies to our own writes too).
    std::fs::write(&path, &json).map_err(|e| BridgeRuntimeError::Io(e.to_string()))?;
    let written =
        std::fs::read_to_string(&path).map_err(|e| BridgeRuntimeError::Io(e.to_string()))?;
    if written != json {
        return Err(BridgeRuntimeError::Io("config read-back mismatch".into()));
    }
    if !written.contains("\"127.0.0.1\"") {
        return Err(BridgeRuntimeError::Io(
            "config invariant violated: host is not loopback".into(),
        ));
    }
    Ok(path)
}

/// Is the pid in the state dir alive? argv-only probe (`kill -0`), no shell.
fn pid_alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// T52 status: read the pid file and probe liveness.
pub fn status(state_dir: &Path) -> BridgeStatus {
    let pf = pid_file(state_dir);
    let pid = std::fs::read_to_string(&pf)
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
        .filter(|p| pid_alive(*p));
    BridgeStatus {
        running: pid.is_some(),
        pid,
        gateway_port: GATEWAY_PORT,
        version: PINNED.version.to_string(),
    }
}

/// T52/T53: start `ccr serve` detached (own process group), argv-only, with
/// the pid recorded for cross-restart status. Returns the child pid.
pub fn start(node: &Path, install_dir: &Path, state_dir: &Path) -> Result<u32, BridgeRuntimeError> {
    if !consented_marker(state_dir) {
        return Err(BridgeRuntimeError::NotConsented);
    }
    let cli = install_dir.join("dist").join("main").join("cli.js");
    if !cli.is_file() {
        return Err(BridgeRuntimeError::NotInstalled);
    }
    if status(state_dir).running {
        return Ok(status(state_dir).pid.expect("running ⇒ pid"));
    }

    let mut cmd = Command::new(node);
    cmd.arg(&cli)
        .arg("serve")
        .arg("--host")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(GATEWAY_PORT.to_string())
        .arg("--no-open")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // Detach: survives the GUI closing (T53). Unix-only in v1 (macOS-first).
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            cmd.pre_exec(|| {
                libc_setsid();
                Ok(())
            });
        }
    }
    let child = cmd
        .spawn()
        .map_err(|e| BridgeRuntimeError::Io(format!("spawn: {e}")))?;
    let pid = child.id();
    let mut pf = std::fs::File::create(pid_file(state_dir))
        .map_err(|e| BridgeRuntimeError::Io(e.to_string()))?;
    writeln!(pf, "{pid}").map_err(|e| BridgeRuntimeError::Io(e.to_string()))?;
    Ok(pid)
}

#[cfg(unix)]
fn libc_setsid() {
    // setsid() detaches from the controlling terminal / process group.
    unsafe extern "C" {
        fn setsid() -> i32;
    }
    unsafe {
        let _ = setsid();
    }
}

/// T52 stop: SIGTERM the pid, then remove the pid file.
pub fn stop(state_dir: &Path) -> Result<bool, BridgeRuntimeError> {
    let pf = pid_file(state_dir);
    let pid = std::fs::read_to_string(&pf)
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok());
    let was_running = match pid {
        Some(p) if pid_alive(p) => {
            let _ = Command::new("kill")
                .arg(p.to_string())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            true
        }
        _ => false,
    };
    let _ = std::fs::remove_file(&pf);
    Ok(was_running)
}

/// T48: the consent marker — created only by the consent command.
pub fn write_consent_marker(state_dir: &Path) -> Result<PathBuf, BridgeRuntimeError> {
    std::fs::create_dir_all(state_dir).map_err(|e| BridgeRuntimeError::Io(e.to_string()))?;
    let path = super::installer::consent_marker_path(state_dir);
    let doc = serde_json::json!({
        "consented": true,
        "version": PINNED.version,
        "package": PINNED.package,
        "license": PINNED.license,
        "at": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    });
    std::fs::write(&path, serde_json::to_string_pretty(&doc).expect("json"))
        .map_err(|e| BridgeRuntimeError::Io(e.to_string()))?;
    Ok(path)
}

pub fn consented_marker(state_dir: &Path) -> bool {
    super::installer::consent_marker_path(state_dir).is_file()
}

/// T48: remove the consent marker (consent withdrawal ⇒ runtime refuses).
pub fn clear_consent_marker(state_dir: &Path) {
    let _ = std::fs::remove_file(super::installer::consent_marker_path(state_dir));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ccm-bridge-rt-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    #[test]
    fn config_is_loopback_only_and_keyless() {
        let cfg = generate_gateway_config(
            &GatewayProviderConfig {
                name: "nvidia-nim".into(),
                base_url: "https://integrate.api.nvidia.com".into(),
                api_key_file: "/Users/x/Library/App/secrets/nvidia-nim/cred-1.secret".into(),
            },
            GATEWAY_PORT,
        )
        .expect("config");
        assert!(cfg.contains("\"127.0.0.1\""));
        assert!(
            !cfg.contains("nvapi-"),
            "key material must never be inlined"
        );
        assert!(cfg.contains("api_key_file"));
    }

    #[test]
    fn config_refuses_non_https_remote() {
        let err = generate_gateway_config(
            &GatewayProviderConfig {
                name: "x".into(),
                base_url: "http://evil.example.com".into(),
                api_key_file: "/tmp/k".into(),
            },
            GATEWAY_PORT,
        )
        .expect_err("http remote refused");
        assert!(matches!(err, BridgeRuntimeError::Config(_)));
    }

    #[test]
    fn write_read_back_enforces_loopback() {
        let dir = temp_dir("writer");
        let path = write_gateway_config(
            &dir,
            &GatewayProviderConfig {
                name: "custom".into(),
                base_url: "https://gw.example.com".into(),
                api_key_file: "/tmp/k".into(),
            },
            GATEWAY_PORT,
        )
        .expect("write");
        let body = std::fs::read_to_string(path).expect("read");
        assert!(body.contains("\"port\": 3456"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn status_without_pid_file_is_stopped() {
        let dir = temp_dir("status");
        let st = status(&dir);
        assert!(!st.running);
        assert_eq!(st.pid, None);
        assert_eq!(st.version, PINNED.version);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stop_removes_pid_file_even_when_stale() {
        let dir = temp_dir("stop");
        std::fs::write(pid_file(&dir), b"999999999\n").expect("stale pid");
        let was = stop(&dir).expect("stop");
        assert!(!was, "stale pid was not running");
        assert!(!pid_file(&dir).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn start_refuses_without_consent_marker() {
        let dir = temp_dir("noconsent");
        let err = start(Path::new("/usr/bin/env"), dir.join("v").as_path(), &dir)
            .expect_err("consent gate");
        assert_eq!(err, BridgeRuntimeError::NotConsented);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn start_refuses_when_not_installed() {
        let dir = temp_dir("noinstall");
        write_consent_marker(&dir).expect("consent");
        let err = start(
            Path::new("/usr/bin/env"),
            dir.join("bridge/3.1.3").as_path(),
            &dir,
        )
        .expect_err("not installed");
        assert_eq!(err, BridgeRuntimeError::NotInstalled);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn consent_marker_round_trip() {
        let dir = temp_dir("consent");
        assert!(!consented_marker(&dir));
        write_consent_marker(&dir).expect("write");
        assert!(consented_marker(&dir));
        clear_consent_marker(&dir);
        assert!(!consented_marker(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
