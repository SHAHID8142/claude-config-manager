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
    /// `ANTHROPIC_*` variables exported by the login shell (names + lengths only,
    /// never values). A GUI app cannot see the shell environment, but Claude Code
    /// can — and those variables outrank `apiKeyHelper`, so a stale export silently
    /// overrides everything this app writes. Found from a real 401 report.
    pub shell_env: Vec<ShellEnvVar>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellEnvVar {
    pub name: String,
    pub length: usize,
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
fn find_claude_cli(home: &Path) -> Option<PathBuf> {
    let path_var = std::env::var("PATH").unwrap_or_default();
    // PATH first (a user who installed it there expects that to win), then the
    // real install locations. A GUI app started from Finder/Dock gets only
    // `/usr/bin:/bin:/usr/sbin:/sbin`, so relying on PATH alone reported "Claude
    // Code CLI not found" on machines where it is installed in ~/.local/bin.
    find_claude_cli_in_path(&path_var)
        .or_else(|| {
            cli_candidate_paths(home)
                .into_iter()
                .find(|p| is_executable_file(p))
        })
        .or_else(|| find_claude_cli_extra_path(&path_var, home))
}

/// Common install locations for the Claude Code CLI.
fn cli_candidate_paths(home: &Path) -> Vec<PathBuf> {
    let mut out = vec![
        home.join(".local/bin/claude"),
        home.join(".claude/local/claude"),
        home.join(".npm-global/bin/claude"),
        home.join(".bun/bin/claude"),
        home.join(".volta/bin/claude"),
        PathBuf::from("/opt/homebrew/bin/claude"),
        PathBuf::from("/usr/local/bin/claude"),
        PathBuf::from("/usr/bin/claude"),
    ];
    // Node-version managers: ~/.nvm/versions/node/<ver>/bin/claude and fnm.
    for base in [
        home.join(".nvm/versions/node"),
        home.join(".fnm/node-versions"),
    ] {
        if let Ok(entries) = std::fs::read_dir(&base) {
            let mut versions: Vec<PathBuf> =
                entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
            versions.sort();
            for v in versions.into_iter().rev() {
                out.push(v.join("bin/claude"));
                out.push(v.join("installation/bin/claude"));
            }
        }
    }
    out
}

/// Paths the version probe itself needs: a GUI app has no Homebrew prefix, so a
/// `#!/usr/bin/env node` script would fail even when we found it.
fn probe_path(home: &Path, cli: &Path) -> String {
    let mut dirs: Vec<String> = Vec::new();
    if let Some(dir) = cli.parent() {
        dirs.push(dir.to_string_lossy().to_string());
    }
    for d in [
        home.join(".local/bin"),
        home.join(".npm-global/bin"),
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/bin"),
        PathBuf::from("/bin"),
    ] {
        dirs.push(d.to_string_lossy().to_string());
    }
    if let Ok(existing) = std::env::var("PATH") {
        dirs.push(existing);
    }
    dirs.join(":")
}

fn is_executable_file(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(p)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// Last resort: ask the user's login shell (PATH from `.zprofile` etc. that a GUI
/// app never inherits). Only the resolved path is used; output is discarded.
fn find_claude_cli_extra_path(path_var: &str, home: &Path) -> Option<PathBuf> {
    for shell in ["/bin/zsh", "/bin/bash"] {
        if !Path::new(shell).exists() {
            continue;
        }
        let out = std::process::Command::new(shell)
            .args(["-lc", "command -v claude"])
            .env("PATH", path_var)
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output()
            .ok()?;
        if !out.status.success() {
            continue;
        }
        let text = String::from_utf8_lossy(&out.stdout);
        let candidate = PathBuf::from(text.lines().next().unwrap_or("").trim());
        if is_executable_file(&candidate) && candidate.starts_with(home) {
            return Some(candidate);
        }
    }
    None
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
/// `path` is injected for tests; production always builds a usable one.
fn probe_cli_version(cli: &Path, path: Option<&str>) -> Result<Option<String>, String> {
    let mut cmd = Command::new(cli);
    cmd.arg("--version");
    if let Some(p) = path {
        cmd.env("PATH", p);
    }
    let mut child = cmd
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

/// Variables that outrank `apiKeyHelper`/`settings.model` when exported.
const SHADOWABLE_ENV: &[&str] = &[
    "ANTHROPIC_AUTH_TOKEN",
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_BASE_URL",
    "ANTHROPIC_MODEL",
];

/// Parse `NAME=len` lines produced by the probe shell (never values).
fn parse_shell_env(output: &str) -> Vec<ShellEnvVar> {
    let mut out = Vec::new();
    for line in output.lines() {
        let Some((name, len)) = line.trim().split_once('=') else {
            continue;
        };
        if !SHADOWABLE_ENV.contains(&name) {
            continue;
        }
        if let Ok(length) = len.trim().parse::<usize>() {
            // An exported-but-empty variable still counts as present.
            out.push(ShellEnvVar {
                name: name.to_string(),
                length,
            });
        }
    }
    out
}

/// Ask the user's login shell which shadowable variables it exports.
///
/// Only names and lengths are requested, so no credential can end up in a log,
/// in the UI, or in an error message. Runs with a short timeout: this is a
/// diagnostic, never a blocker.
fn probe_shell_env() -> Vec<ShellEnvVar> {
    let script = format!(
        r#"for v in {}; do eval "val=\$$v"; if [ -n "${{val+x}}" ]; then echo "$v=${{#val}}"; fi; done"#,
        SHADOWABLE_ENV.join(" ")
    );
    for shell in ["/bin/zsh", "/bin/bash"] {
        if !Path::new(shell).exists() {
            continue;
        }
        let mut child = match Command::new(shell)
            .args(["-lic", &script])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(_) => continue,
        };
        let started = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(st)) => break Some(st),
                Ok(None) if started.elapsed() < Duration::from_secs(3) => {
                    std::thread::sleep(Duration::from_millis(25));
                }
                _ => {
                    let _ = child.kill();
                    break None;
                }
            }
        };
        if status.is_some_and(|s| s.success()) {
            let mut buf = String::new();
            if let Some(mut out) = child.stdout.take() {
                use std::io::Read;
                let _ = out.read_to_string(&mut buf);
            }
            let parsed = parse_shell_env(&buf);
            if !parsed.is_empty() {
                return parsed;
            }
        }
    }
    Vec::new()
}

/// Build the full environment report.
pub fn check_environment(home: &Path, config_dir_override: Option<&Path>) -> EnvironmentReport {
    // CLI detection + version probe.
    let (cli_found, cli_version, cli_version_warning) = match find_claude_cli(home) {
        Some(cli) => {
            // A `#!/usr/bin/env node` script needs node on PATH; the GUI PATH
            // does not have Homebrew or nvm, so supply one.
            let v = probe_cli_version(&cli, Some(&probe_path(home, &cli)));
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
        shell_env: probe_shell_env(),
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
    use std::os::unix::fs::PermissionsExt;

    /// A GUI app started from Finder inherits only `/usr/bin:/bin:/usr/sbin:/sbin`,
    /// so PATH-only detection reported "not found" on working installs.
    #[test]
    fn cli_is_found_in_the_usual_install_locations_without_a_shell_path() {
        let home = std::env::temp_dir().join(format!(
            "ccm-cli-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(home.join(".local/bin")).expect("dir");
        let cli = home.join(".local/bin/claude");
        std::fs::write(&cli, "#!/bin/sh\necho 2.0.0\n").expect("write");
        std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o700)).expect("chmod");

        // PATH as seen by a GUI app: our install location is absent.
        assert_eq!(
            find_claude_cli_in_path("/usr/bin:/bin:/usr/sbin:/sbin"),
            None
        );
        let found = cli_candidate_paths(&home)
            .into_iter()
            .find(|p| is_executable_file(p));
        assert_eq!(
            found.as_deref(),
            Some(cli.as_path()),
            "must fall back to known locations"
        );

        // A non-executable file with that name is not a usable CLI.
        std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o600)).expect("chmod");
        assert!(
            cli_candidate_paths(&home)
                .into_iter()
                .all(|p| !is_executable_file(&p)),
            "a non-executable candidate must not count as found"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_version_probe_gets_a_path_that_can_find_node() {
        // The helper scripts are `#!/usr/bin/env node`; probing with the GUI PATH
        // fails with ENOENT even though the binary exists.
        assert!(
            probe_path(
                Path::new("/Users/x"),
                Path::new("/Users/x/.local/bin/claude")
            )
            .contains("/Users/x/.local/bin")
        );
        assert!(
            probe_path(Path::new("/Users/x"), Path::new("/opt/homebrew/bin/claude"))
                .contains("/opt/homebrew/bin")
        );
    }

    /// A shell export outranks `apiKeyHelper`, so the app must be able to say so
    /// — without ever reading the value.
    #[test]
    fn shell_exports_are_reported_by_name_and_length_only() {
        let parsed = parse_shell_env(
            "ANTHROPIC_AUTH_TOKEN=36\nANTHROPIC_API_KEY=0\nPATH=123\nnot a line\nANTHROPIC_MODEL=4\n",
        );
        assert_eq!(
            parsed,
            vec![
                ShellEnvVar {
                    name: "ANTHROPIC_AUTH_TOKEN".into(),
                    length: 36
                },
                ShellEnvVar {
                    name: "ANTHROPIC_API_KEY".into(),
                    length: 0
                },
                ShellEnvVar {
                    name: "ANTHROPIC_MODEL".into(),
                    length: 4
                },
            ],
            "unrelated variables and malformed lines are ignored"
        );
    }

    #[test]
    fn no_exports_produce_an_empty_report() {
        assert!(parse_shell_env("").is_empty());
    }

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
