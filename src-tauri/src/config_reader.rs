//! Strict-JSON reader for the global Claude Code settings file (T27).
//!
//! Contract (T02 matrix, PRD F01/F02):
//! - Strict JSON only: comments and trailing commas are syntax errors —
//!   serde_json enforces this by default.
//! - **Never overwrite invalid source**: malformed input produces
//!   `MalformedJson`, and nothing downstream may write to that file until a
//!   human resolves it.
//! - Unrelated keys are preserved: documents parse to `serde_json::Value`
//!   with `preserve_order`, so key order and unknown keys survive any later
//!   targeted merge (T28).
//! - Symlinks refused (threat T5) — the T26 locator's `Symlink` status is
//!   an error here, never a read target.

use crate::config_locator::{ConfigLocator, LocatedStatus};
use serde_json::Value;
use std::path::Path;

/// Errors for config read/merge flows. Display strings are user-mappable and
/// redaction-safe (no file content is ever embedded).
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigError {
    Missing,
    PermissionDenied(String),
    MalformedJson(String),
    /// Top-level JSON is valid but not an object.
    MalformedShape(&'static str),
    SymlinkRefused,
    Io(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Missing => write!(f, "settings file does not exist yet"),
            ConfigError::PermissionDenied(m) => write!(f, "settings file not readable ({m})"),
            ConfigError::MalformedJson(_) => {
                write!(
                    f,
                    "settings file contains invalid JSON — not overwritten; fix it manually first"
                )
            }
            ConfigError::MalformedShape(k) => {
                write!(f, "settings file must contain a JSON object, found {k}")
            }
            ConfigError::SymlinkRefused => {
                write!(f, "settings path is a symlink — refused for safety")
            }
            ConfigError::Io(m) => write!(f, "settings file error ({m})"),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Read the global settings file as a JSON object value.
pub fn read_settings(path: &Path) -> Result<Value, ConfigError> {
    let locator = ConfigLocator::new();
    match locator.inspect(path) {
        Ok(LocatedStatus::RegularFile) => {}
        Ok(LocatedStatus::Missing) => return Err(ConfigError::Missing),
        Ok(LocatedStatus::Symlink) => return Err(ConfigError::SymlinkRefused),
        Ok(LocatedStatus::OtherFile) => {
            return Err(ConfigError::Io("not a regular file".to_string()));
        }
        Err(e) => return Err(ConfigError::Io(e.to_string())),
    }

    let raw = std::fs::read_to_string(path).map_err(|e| match e.kind() {
        std::io::ErrorKind::PermissionDenied => ConfigError::PermissionDenied(e.to_string()),
        _ => ConfigError::Io(e.to_string()),
    })?;

    let value: Value = serde_json::from_str(&raw).map_err(|e| {
        // Content snippets are never included — position info only.
        ConfigError::MalformedJson(format!("line {} column {}", e.line(), e.column()))
    })?;

    match value {
        Value::Object(_) => Ok(value),
        other => Err(ConfigError::MalformedShape(shape_name(&other))),
    }
}

fn shape_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

/// Keys in the document that this app does not manage — these must survive
/// any merge unchanged (PRD F02).
pub fn unrelated_keys(doc: &Value, managed_keys: &[&str]) -> Vec<String> {
    match doc {
        Value::Object(map) => map
            .keys()
            .filter(|k| !managed_keys.contains(&k.as_str()))
            .cloned()
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::fs::symlink;
    use std::path::PathBuf;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ccm-t27-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    const MANAGED: &[&str] = &["model", "env", "apiKeyHelper"];

    #[test]
    fn reads_valid_document() {
        let dir = temp_dir("valid");
        let path = dir.join("settings.json");
        fs::write(&path, r#"{"model":"claude-x","customKey":{"nested":true}}"#).expect("write");
        let doc = read_settings(&path).expect("read");
        assert_eq!(doc["model"], "claude-x");
        assert_eq!(doc["customKey"]["nested"], true);
        fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn missing_file_reports_missing() {
        let dir = temp_dir("missing");
        let path = dir.join("settings.json");
        assert_eq!(read_settings(&path), Err(ConfigError::Missing));
        fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn symlink_is_refused() {
        let dir = temp_dir("symlink");
        let path = dir.join("settings.json");
        let victim = dir.join("victim.txt");
        fs::write(&victim, "sensitive").expect("victim");
        symlink(&victim, &path).expect("symlink");
        assert_eq!(read_settings(&path), Err(ConfigError::SymlinkRefused));
        fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn malformed_json_reports_position_without_content_leak() {
        let dir = temp_dir("malformed");
        let path = dir.join("settings.json");
        fs::write(&path, "{\n  \"model\": \"x\",\n}").expect("write trailing comma");
        let err = read_settings(&path).expect_err("trailing comma is an error");
        match err {
            ConfigError::MalformedJson(pos) => {
                assert!(pos.contains("line"), "position info: {pos}");
                assert!(!pos.contains("model"), "no content in error: {pos}");
            }
            other => panic!("expected MalformedJson, got {other:?}"),
        }
        fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn json_comments_are_rejected_strict() {
        let dir = temp_dir("comments");
        let path = dir.join("settings.json");
        fs::write(&path, "{\n  // not allowed\n  \"model\": \"x\"\n}").expect("write");
        assert!(matches!(
            read_settings(&path),
            Err(ConfigError::MalformedJson(_))
        ));
        fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn non_object_root_rejected() {
        let dir = temp_dir("array");
        let path = dir.join("settings.json");
        fs::write(&path, "[1, 2, 3]").expect("write");
        assert_eq!(
            read_settings(&path),
            Err(ConfigError::MalformedShape("an array"))
        );
        fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn unrelated_keys_survive_round_trip() {
        let dir = temp_dir("preserve");
        let path = dir.join("settings.json");
        fs::write(
            &path,
            r#"{"statusLine":{"type":"command"},"model":"m","permissions":{"allow":[]},"env":{}}"#,
        )
        .expect("write");
        let doc = read_settings(&path).expect("read");
        let unrelated = unrelated_keys(&doc, MANAGED);
        assert_eq!(
            unrelated,
            vec!["statusLine".to_string(), "permissions".to_string()]
        );
        // Round-trip: unrelated keys still present verbatim after re-serialize.
        let round = serde_json::to_string(&doc).expect("serialize");
        assert!(round.contains("\"statusLine\""));
        assert!(round.contains("\"permissions\""));
        fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn key_order_is_preserved_across_round_trip() {
        let dir = temp_dir("order");
        let path = dir.join("settings.json");
        fs::write(&path, r#"{"zebra":1,"alpha":2,"model":"m"}"#).expect("write");
        let doc = read_settings(&path).expect("read");
        let round = serde_json::to_string(&doc).expect("serialize");
        let zebra_pos = round.find("\"zebra\"").expect("zebra");
        let alpha_pos = round.find("\"alpha\"").expect("alpha");
        assert!(
            zebra_pos < alpha_pos,
            "insertion order must survive (preserve_order): {round}"
        );
        fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn permission_denied_reports_perm_error() {
        let dir = temp_dir("perm");
        let path = dir.join("settings.json");
        fs::write(&path, "{}").expect("write");
        fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).expect("chmod");
        let result = read_settings(&path);
        // Restore before cleanup so remove_dir_all works.
        fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).expect("restore");
        match result {
            Err(ConfigError::PermissionDenied(_)) => {}
            other => panic!("expected PermissionDenied, got {other:?}"),
        }
        fs::remove_dir_all(&dir).expect("cleanup");
    }
}
