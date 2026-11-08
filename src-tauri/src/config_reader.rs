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
    /// The document exceeds `SETTINGS_MAX_BYTES` (G5). Carries the size only —
    /// never any of the content.
    TooLarge {
        size: u64,
        limit: u64,
    },
    /// Invalid JSON whose cause can be named safely (G6): comments or a trailing
    /// comma. `hint` is a fixed string, never file content.
    MalformedJsonHint(&'static str),
    /// A value failed schema validation before any write (T28 gate).
    ValidationFailed(String),
    /// The file changed between snapshot and write — another process edited it.
    ConcurrentModification,
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
            ConfigError::MalformedJsonHint(hint) => write!(
                f,
                "settings file contains invalid JSON ({hint}) — not overwritten; fix it manually first"
            ),
            ConfigError::TooLarge { size, limit } => write!(
                f,
                "the settings file is {size} bytes, larger than the {limit} byte limit — refusing to touch it"
            ),
            ConfigError::MalformedShape(k) => {
                write!(f, "settings file must contain a JSON object, found {k}")
            }
            ConfigError::ValidationFailed(m) => write!(f, "{m}"),
            ConfigError::ConcurrentModification => write!(
                f,
                "settings file changed while you were editing — review and retry"
            ),
            ConfigError::SymlinkRefused => {
                write!(f, "settings path is a symlink — refused for safety")
            }
            ConfigError::Io(m) => write!(f, "settings file error ({m})"),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Read the global settings file as a JSON object value.
/// Hard cap on the settings document (G5).
///
/// Real files are a few KiB. A multi-megabyte `settings.json` is either not a
/// settings file or is hostile, and reading it into memory before discovering
/// that is exactly the kind of avoidable failure this app refuses to have.
pub const SETTINGS_MAX_BYTES: u64 = 8 * 1024 * 1024;

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
    if locator
        .config_dir_is_symlink(path)
        .map_err(|e| ConfigError::Io(e.to_string()))?
    {
        return Err(ConfigError::SymlinkRefused);
    }

    // Size gate before reading (G5): refuse rather than allocate first.
    let size = std::fs::metadata(path)
        .map_err(|e| ConfigError::Io(e.to_string()))?
        .len();
    if size > SETTINGS_MAX_BYTES {
        return Err(ConfigError::TooLarge {
            size,
            limit: SETTINGS_MAX_BYTES,
        });
    }

    let raw = std::fs::read_to_string(path).map_err(|e| match e.kind() {
        std::io::ErrorKind::PermissionDenied => ConfigError::PermissionDenied(e.to_string()),
        _ => ConfigError::Io(e.to_string()),
    })?;

    let value: Value = serde_json::from_str(&raw).map_err(|e| {
        // G6: name the most likely cause when we can do so safely. Claude Code
        // does not accept comments in settings.json (code.claude.com/docs/en/
        // settings-example), so a "helpful" lenient parser here would silently
        // normalise a file Claude Code itself then rejects.
        if let Some(hint) = dialect_hint(&raw) {
            ConfigError::MalformedJsonHint(hint)
        } else {
            // Content snippets are never included — position info only.
            ConfigError::MalformedJson(format!("line {} column {}", e.line(), e.column()))
        }
    })?;

    match value {
        Value::Object(_) => Ok(value),
        other => Err(ConfigError::MalformedShape(shape_name(&other))),
    }
}

/// Identify a JSON dialect mistake without quoting file content.
///
/// Scans outside strings only, so `"https://x"` never looks like a comment.
/// Returns a fixed description or `None`.
pub fn dialect_hint(raw: &str) -> Option<&'static str> {
    let bytes = raw.as_bytes();
    let mut in_string = false;
    let mut escaped = false;
    let mut last_significant: Option<u8> = None;
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if in_string {
            if escaped {
                escaped = false;
            } else if c == b'\\' {
                escaped = true;
            } else if c == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        match c {
            b'"' => {
                in_string = true;
                last_significant = Some(c);
            }
            b'/' if i + 1 < bytes.len() && (bytes[i + 1] == b'/' || bytes[i + 1] == b'*') => {
                return Some("comments are not allowed in settings.json");
            }
            b' ' | b'\t' | b'\r' | b'\n' => {}
            b'}' | b']' => {
                if last_significant == Some(b',') {
                    return Some("trailing comma");
                }
                last_significant = Some(c);
            }
            _ => last_significant = Some(c),
        }
        i += 1;
    }
    None
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
    use std::time::SystemTime;

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

    /// G6: Claude Code rejects comments in settings.json (official docs), so we
    /// must not "helpfully" accept them — but the error should name the cause.
    #[test]
    fn comments_are_refused_with_a_cause_not_a_generic_error() {
        let dir = std::env::temp_dir().join(format!(
            "ccm-g6-comment-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("settings.json");
        fs::write(
            &path,
            "{\n  // why: needed for the proxy\n  \"model\": \"claude-sonnet-4-5\"\n}",
        )
        .expect("write");

        let err = read_settings(&path).expect_err("comments must be refused");
        assert!(
            matches!(err, ConfigError::MalformedJsonHint(h) if h.contains("comments")),
            "expected a comment hint, got {err:?}"
        );
        assert!(err.to_string().contains("fix it manually"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn trailing_comma_is_refused_and_named() {
        let dir = std::env::temp_dir().join(format!(
            "ccm-g6-comma-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("settings.json");
        fs::write(&path, r#"{"model":"x","env":{"A":"1",},}"#).expect("write");

        let err = read_settings(&path).expect_err("trailing comma must be refused");
        assert!(
            matches!(err, ConfigError::MalformedJsonHint(h) if h.contains("trailing comma")),
            "expected a trailing-comma hint, got {err:?}"
        );
        fs::remove_dir_all(&dir).ok();
    }

    /// The hint must never fire on content inside strings — URLs contain `//`.
    #[test]
    fn comment_markers_inside_strings_are_not_mistaken_for_comments() {
        assert_eq!(
            dialect_hint(r#"{"a":"https://example.com/x","b":"/* not a comment */"}"#),
            None
        );
        assert_eq!(dialect_hint(r#"{"a":1}"#), None);
        assert_eq!(dialect_hint(r#"{"a":1,}"#), Some("trailing comma"));
        assert_eq!(
            dialect_hint(r#"{"a":1} // tail"#),
            Some("comments are not allowed in settings.json")
        );
    }

    /// G5: an oversized document is refused before it is read into memory.
    #[test]
    fn oversized_settings_file_is_refused_with_the_size_in_the_message() {
        use std::io::Write;
        let dir = std::env::temp_dir().join(format!(
            "ccm-g5-size-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("settings.json");
        let mut f = fs::File::create(&path).expect("create");
        f.write_all(br#"{"model":"#).expect("prefix");
        // One byte over the cap; content validity is irrelevant, size is checked first.
        let filler = vec![b' '; (SETTINGS_MAX_BYTES + 1) as usize];
        f.write_all(&filler).expect("filler");
        f.write_all(br#""x"}"#).expect("suffix");
        f.sync_all().expect("sync");
        drop(f);

        let err = read_settings(&path).expect_err("must refuse");
        assert!(
            matches!(err, ConfigError::TooLarge { size, limit }
                if size > limit && limit == SETTINGS_MAX_BYTES),
            "expected TooLarge, got {err:?}"
        );
        let msg = err.to_string();
        assert!(msg.contains("byte limit"), "unexpected message: {msg}");
        assert!(msg.contains(&SETTINGS_MAX_BYTES.to_string()));
        fs::remove_dir_all(&dir).ok();
    }

    /// G5: a deeply nested document must produce an error, not a stack overflow.
    #[test]
    fn deeply_nested_json_is_rejected_not_a_stack_overflow() {
        let dir = std::env::temp_dir().join(format!(
            "ccm-g5-nest-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("settings.json");
        let depth = 5_000;
        let mut text = String::from("{\"a\":");
        text.push_str(&"[".repeat(depth));
        text.push_str(&"]".repeat(depth));
        text.push('}');
        fs::write(&path, &text).expect("write");

        let err = read_settings(&path).expect_err("deep nesting must be refused");
        assert!(
            matches!(err, ConfigError::MalformedJson(_)),
            "expected a parse error, got {err:?}"
        );
        fs::remove_dir_all(&dir).ok();
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
        // A genuine syntax error, not one of the named dialects (those get a
        // dedicated hint since G6) — `:` where a value is required.
        fs::write(&path, "{\n  \"model\": : \"x\"\n}").expect("write malformed");
        let err = read_settings(&path).expect_err("malformed JSON is an error");
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
        // Rejection is the contract; since G6 the error names the cause.
        assert!(matches!(
            read_settings(&path),
            Err(ConfigError::MalformedJson(_)) | Err(ConfigError::MalformedJsonHint(_))
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
