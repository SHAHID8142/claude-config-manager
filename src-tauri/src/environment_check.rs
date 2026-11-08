//! Environment diagnostics (T36, PRD F06/F36): explicit, honest failure
//! handling for missing CLI, unparseable versions and invalid settings.
//!
//! Rules:
//! - The CLI is invoked argv-only (`claude --version`), never through a
//!   shell, never with user input (threat T6); output is capped by a short
//!   poll-based timeout.
//! - No invented version requirements (T02): we REPORT the version and flag
//!   only unparseable output as "unknown" — never claim incompatibility we
//!   cannot verify.
//! - Settings state reuses the T27/T34 machinery: malformed sources are
//!   reported as "do not overwrite" states, never auto-repaired.

use crate::config_locator::{ConfigLocator, LocatedStatus};
use crate::config_reader::read_settings;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentReport {
    pub cli_found: bool,
    pub cli_version: Option<String>,
    /// Set when the CLI exists but its version output was unparseable.
    pub cli_version_warning: Option<String>,
    /// One of: ok | first_run | malformed_json | not_a_file | unreadable
    pub settings_state: String,
    /// Human detail for non-ok settings states (no content echoed).
    pub settings_detail: Option<String>,
    pub config_dir_writable: bool,
}

/// Extract an `x.y.z`-shaped token from a version output line.
/// Unit-tested directly; deliberately permissive (report, don't judge).
pub fn parse_version_output(output: &str) -> Option<String> {
    let first_line = output.lines().next().unwrap_or("");
    for token in first_line.split_whitespace() {
        let core = token.trim_start_matches('v');
        let parts: Vec<&str> = core.split('.').collect();
        if parts.len() >= 2
            && parts
                .iter()
                .take(2)
                .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
        {
            return Some(core.to_string());
        }
    }
    None
}

/// Locate the `claude` binary without executing it.
fn find_claude_cli() -> Option<PathBuf> {
    let path_var = std::env::var("PATH").unwrap_or_default();
    find_claude_cli_in_path(&path_var)
}

/// Testable core: search a PATH string for the `claude` binary.
fn find_claude_cli_in_path(path_var: &str) -> Option<PathBuf> {
    for dir in path_var.split(':') {
        if dir.is_empty() {
            continue;
        }
        let candidate = Path::new(dir).join("claude");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Run `claude --version` with a poll timeout (argv-only, no shell).
fn probe_cli_version(cli: &Path) -> Result<Option<String>, String> {
    let mut child = Command::new(cli)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("spawn failed: {e}"))?;

    let start = Instant::now();
    let timeout = Duration::from_secs(5);
    let status = loop {
        match child.try_wait() {
            Ok(Some(st)) => break Ok(st),
            Ok(None) if start.elapsed() < timeout => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Ok(None) => {
                let _ = child.kill();
                break Err("timeout".to_string());
            }
            Err(e) => break Err(format!("wait failed: {e}")),
        }
    };

    use std::io::Read;
    let mut out = String::new();
    if let Some(mut stdout) = child.stdout.take() {
        let _ = stdout.read_to_string(&mut out);
    }

    match status {
        Ok(st) if st.success() => Ok(parse_version_output(&out)),
        Ok(_) => Err("non-zero exit".to_string()),
        Err(m) => Err(m),
    }
}

/// Probe write access to the config dir without writing (access(2) via
/// metadata checks is insufficient for dirs — use a no-op open attempt on
/// the dir itself, which is read; instead check the parent-ancestor rule:
/// creating a temp probe file is the reliable test, cleaned up immediately).
fn config_dir_writable(settings_path: &Path) -> bool {
    let Some(dir) = settings_path.parent() else {
        return false;
    };
    if !dir.is_dir() {
        // First-run: check the nearest existing ancestor chain by attempting
        // creation lazily is destructive-ish; report based on what exists.
        return false;
    }
    let probe = dir.join(format!(
        ".ccm-writable-probe-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
    {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

/// Build the full environment report.
pub fn check_environment(home: &Path, config_dir_override: Option<&Path>) -> EnvironmentReport {
    // CLI detection + version probe.
    let (cli_found, cli_version, cli_version_warning) = match find_claude_cli() {
        Some(cli) => {
            let v = probe_cli_version(&cli);
            match v {
                Ok(Some(ver)) => (true, Some(ver), None),
                Ok(None) => (
                    true,
                    None,
                    Some("CLI found but version output was unparseable".into()),
                ),
                Err(m) => (true, None, Some(format!("CLI probe failed ({m})"))),
            }
        }
        None => (
            false,
            None,
            Some(
                "Claude Code CLI not found on PATH — settings can still be managed, but install the CLI to use them".into(),
            ),
        ),
    };

    // Settings state (read-only diagnostics).
    let locator = ConfigLocator::new();
    let settings_path = locator.locate(home, config_dir_override);
    let (settings_state, settings_detail) = match locator.inspect(&settings_path) {
        Ok(LocatedStatus::Missing) => ("first_run".to_string(), None),
        Ok(LocatedStatus::RegularFile) => match read_settings(&settings_path) {
            Ok(_) => ("ok".to_string(), None),
            Err(e) => (state_name(&e).to_string(), Some(e.to_string())),
        },
        Ok(LocatedStatus::Symlink) => (
            "symlink".to_string(),
            Some("settings path is a symlink — refused for safety".into()),
        ),
        Ok(LocatedStatus::OtherFile) => (
            "not_a_file".to_string(),
            Some("settings path is not a regular file".into()),
        ),
        Err(e) => ("unreadable".to_string(), Some(e.to_string())),
    };

    EnvironmentReport {
        cli_found,
        cli_version,
        cli_version_warning,
        settings_state,
        settings_detail,
        config_dir_writable: config_dir_writable(&settings_path),
    }
}

fn state_name(e: &crate::config_reader::ConfigError) -> &'static str {
    match e {
        crate::config_reader::ConfigError::MalformedJson(_) => "malformed_json",
        crate::config_reader::ConfigError::PermissionDenied(_) => "unreadable",
        crate::config_reader::ConfigError::MalformedShape(_) => "malformed_json",
        crate::config_reader::ConfigError::SymlinkRefused => "symlink",
        _ => "unreadable",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_parser_extracts_semver_shaped_tokens() {
        assert_eq!(
            parse_version_output("2.1.246 (Claude Code)"),
            Some("2.1.246".into())
        );
        assert_eq!(parse_version_output("v3.0.1"), Some("3.0.1".into()));
        assert_eq!(
            parse_version_output("claude 1.2.3 abc"),
            Some("1.2.3".into())
        );
    }

    #[test]
    fn version_parser_rejects_garbage_and_empty() {
        assert_eq!(parse_version_output(""), None);
        assert_eq!(parse_version_output("garbage only"), None);
        assert_eq!(parse_version_output("version unknown"), None);
    }

    #[test]
    fn missing_cli_is_reported_honestly() {
        // A PATH string with no `claude` binary — no env mutation needed.
        assert_eq!(find_claude_cli_in_path(""), None);
        assert_eq!(
            find_claude_cli_in_path(std::env::temp_dir().to_string_lossy().as_ref()),
            None,
            "temp dir has no claude binary"
        );
    }

    #[test]
    fn invalid_settings_reported_without_repair() {
        let home = std::env::temp_dir().join(format!(
            "ccm-t36-bad-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(home.join(".claude")).expect("config dir");
        std::fs::write(home.join(".claude/settings.json"), "{broken").expect("bad settings");

        let report = check_environment(&home, None);
        assert_eq!(report.settings_state, "malformed_json");
        assert!(report.settings_detail.is_some());
        // Never echoes file content in the detail.
        assert!(
            !report
                .settings_detail
                .as_ref()
                .expect("detail")
                .contains("broken")
        );

        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn unwritable_config_dir_flagged() {
        use std::os::unix::fs::PermissionsExt;
        let home = std::env::temp_dir().join(format!(
            "ccm-t36-ronw-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let claude = home.join(".claude");
        std::fs::create_dir_all(&claude).expect("config dir");
        std::fs::set_permissions(&claude, std::fs::Permissions::from_mode(0o500)).expect("chmod");
        let report = check_environment(&home, None);
        std::fs::set_permissions(&claude, std::fs::Permissions::from_mode(0o700)).expect("restore");
        assert!(!report.config_dir_writable);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn no_probe_file_left_behind() {
        let home = std::env::temp_dir().join(format!(
            "ccm-t36-probe-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(home.join(".claude")).expect("config dir");
        let _ = check_environment(&home, None);
        let residue: Vec<_> = std::fs::read_dir(home.join(".claude"))
            .expect("dir")
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .contains("ccm-writable-probe")
            })
            .collect();
        assert!(
            residue.is_empty(),
            "probe files must be cleaned: {residue:?}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}
