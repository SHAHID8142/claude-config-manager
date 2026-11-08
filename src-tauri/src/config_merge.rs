//! Targeted configuration merge with schema validation (T28).
//!
//! Contract (T02 matrix §6, PRD F02/F06):
//! - Only the managed key set is touched; every other key — at the top level
//!   AND inside `env` — passes through byte-for-byte.
//! - `None` in `DesiredSettings` means "leave this managed key untouched"
//!   (settings can set but not remove; removal is a separate explicit
//!   operation, not a side effect of a merge).
//! - All desired values are schema-validated BEFORE merging; a rejected
//!   value never yields a half-written document.

use crate::config_reader::ConfigError;
use crate::credential_handoff::validate_provider_id;
use crate::provider_catalog::classify_endpoint;
use serde_json::{Map, Value};
use std::path::Path;

/// The env keys this app manages inside `env` (T02 verified set).
pub const MANAGED_ENV_KEYS: &[&str] = &[
    "ANTHROPIC_BASE_URL",
    "ANTHROPIC_AUTH_TOKEN",
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_MODEL",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL",
    "ANTHROPIC_DEFAULT_SONNET_MODEL",
    "ANTHROPIC_DEFAULT_OPUS_MODEL",
];

/// Top-level keys this app manages.
pub const MANAGED_TOP_KEYS: &[&str] = &["model", "env", "apiKeyHelper"];

/// What the user wants applied. `None` = leave the key untouched.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DesiredSettings {
    pub model: Option<String>,
    pub env_base_url: Option<String>,
    pub env_auth_token: Option<String>,
    pub env_api_key: Option<String>,
    pub env_model: Option<String>,
    pub env_default_haiku: Option<String>,
    pub env_default_sonnet: Option<String>,
    pub env_default_opus: Option<String>,
    /// Absolute path to an installed apiKeyHelper script (T32 helper mode).
    /// Mutually exclusive with `env_auth_token`/`env_api_key`: helper
    /// credentials rank BELOW those env vars in auth precedence, so setting
    /// both would silently shadow the helper (T02 auth matrix).
    pub api_key_helper: Option<String>,
}

impl DesiredSettings {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// Managed env values in key order (skipping untouched ones).
    fn env_writes(&self) -> Vec<(&'static str, &str)> {
        [
            ("ANTHROPIC_BASE_URL", &self.env_base_url),
            ("ANTHROPIC_AUTH_TOKEN", &self.env_auth_token),
            ("ANTHROPIC_API_KEY", &self.env_api_key),
            ("ANTHROPIC_MODEL", &self.env_model),
            ("ANTHROPIC_DEFAULT_HAIKU_MODEL", &self.env_default_haiku),
            ("ANTHROPIC_DEFAULT_SONNET_MODEL", &self.env_default_sonnet),
            ("ANTHROPIC_DEFAULT_OPUS_MODEL", &self.env_default_opus),
        ]
        .into_iter()
        .filter_map(|(k, v)| v.as_ref().map(|s| (k, s.as_str())))
        .collect()
    }

    /// Validate every set value before any write (schema gate).
    pub fn validate(&self) -> Result<(), ConfigError> {
        if let Some(model) = &self.model {
            validate_model_id(model)?;
        }
        // Auth-mode mutual exclusion (T32): the helper must not be shadowed.
        if self.api_key_helper.is_some()
            && (self.env_auth_token.is_some() || self.env_api_key.is_some())
        {
            return Err(ConfigError::ValidationFailed(
                "authentication mode conflict: apiKeyHelper and plaintext env credentials are mutually exclusive (helper credentials rank below env keys and would be shadowed)".into(),
            ));
        }
        if let Some(helper) = &self.api_key_helper {
            let p = Path::new(helper);
            if !p.is_absolute() {
                return Err(ConfigError::ValidationFailed(
                    "apiKeyHelper must be an absolute path (settings env values are not shell-expanded; T02)".into(),
                ));
            }
            if helper.contains('\0') || helper.len() > 4096 {
                return Err(ConfigError::ValidationFailed(
                    "apiKeyHelper: invalid path".into(),
                ));
            }
        }
        for (key, value) in self.env_writes() {
            match key {
                "ANTHROPIC_BASE_URL" => {
                    if matches!(
                        classify_endpoint(value),
                        crate::provider_catalog::EndpointCheck::Invalid(_)
                            | crate::provider_catalog::EndpointCheck::Insecure
                    ) {
                        return Err(ConfigError::ValidationFailed(format!(
                            "{key}: endpoint must be a valid https URL"
                        )));
                    }
                }
                _ => {
                    validate_env_model_value(key, value)?;
                }
            }
        }
        Ok(())
    }
}

/// Model IDs: non-empty, printable ASCII, no whitespace or control chars.
/// (Providers differ widely in ID syntax — we constrain only what is unsafe.)
pub fn validate_model_id(model: &str) -> Result<(), ConfigError> {
    // T60: a pasted API key satisfies "printable ASCII" but would be written to
    // settings.json in plaintext. Refuse credential-shaped values outright.
    let t = model.trim();
    if crate::redact::is_secret_shaped(t) {
        return Err(ConfigError::ValidationFailed(
            "value looks like an API key, not a model id — refusing to write it in plaintext"
                .into(),
        ));
    }
    if t.is_empty() || t.len() > 256 {
        return Err(ConfigError::ValidationFailed(
            "model: empty or oversized".into(),
        ));
    }
    if !t.chars().all(|c| c.is_ascii_graphic() && c != ' ') {
        return Err(ConfigError::ValidationFailed(
            "model: contains whitespace or control characters".into(),
        ));
    }
    Ok(())
}

fn validate_env_model_value(key: &str, value: &str) -> Result<(), ConfigError> {
    if key.starts_with("ANTHROPIC_MODEL") || key.starts_with("ANTHROPIC_DEFAULT_") {
        return validate_model_id(value);
    }
    // Auth keys: non-empty, no control characters (newlines would break headers).
    if value.is_empty() || value.chars().any(|c| c.is_control()) {
        return Err(ConfigError::ValidationFailed(format!(
            "{key}: empty or contains control characters"
        )));
    }
    if value.len() > 4096 {
        return Err(ConfigError::ValidationFailed(format!("{key}: oversized")));
    }
    Ok(())
}

/// Apply the desired settings onto the current document, returning a NEW
/// document (the caller owns backup/atomic-write, T29/T30).
pub fn apply_desired(current: &Value, desired: &DesiredSettings) -> Result<Value, ConfigError> {
    desired.validate()?;

    let Some(current_obj) = current.as_object() else {
        return Err(ConfigError::MalformedShape("the current document"));
    };

    // Start from a clone: everything (managed or not) carries over.
    let mut out = current_obj.clone();

    if let Some(model) = &desired.model {
        out.insert("model".to_string(), Value::String(model.clone()));
    }

    if let Some(helper) = &desired.api_key_helper {
        // Defense in depth: the helper path's provider segment must refer to
        // a valid provider id (the script name pattern is ours, T32).
        let bad_stem = Path::new(helper)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .map(|stem| validate_provider_id(&stem).is_err())
            .unwrap_or(false);
        if bad_stem {
            return Err(ConfigError::ValidationFailed(
                "apiKeyHelper path must point at an installed helper script for a valid provider id".into(),
            ));
        }
        out.insert("apiKeyHelper".to_string(), Value::String(helper.clone()));
    }

    let env_writes = desired.env_writes();
    if !env_writes.is_empty() {
        let existing_env = match out.get("env") {
            Some(Value::Object(m)) => m.clone(),
            Some(Value::Null) | None => Map::new(),
            Some(other) => {
                return Err(ConfigError::ValidationFailed(format!(
                    "existing `env` is {}, refusing to overwrite — fix manually first",
                    match other {
                        Value::Array(_) => "an array",
                        Value::String(_) => "a string",
                        _ => "not an object",
                    }
                )));
            }
        };
        let mut env = existing_env;
        for (key, value) in env_writes {
            env.insert(key.to_string(), Value::String(value.to_string()));
        }
        out.insert("env".to_string(), Value::Object(env));
    }

    Ok(Value::Object(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(pairs: &[(&str, Value)]) -> Value {
        Value::Object(
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        )
    }

    #[test]
    fn merge_writes_model_and_env_into_empty_document() {
        let desired = DesiredSettings {
            model: Some("claude-sonnet-4-5".into()),
            env_base_url: Some("https://api.example.com".into()),
            env_api_key: Some("sk-ant-FAKE-000".into()),
            ..Default::default()
        };
        let out = apply_desired(&obj(&[]), &desired).expect("merge");
        assert_eq!(out["model"], "claude-sonnet-4-5");
        assert_eq!(out["env"]["ANTHROPIC_BASE_URL"], "https://api.example.com");
        assert_eq!(out["env"]["ANTHROPIC_API_KEY"], "sk-ant-FAKE-000");
    }

    #[test]
    fn merge_preserves_unrelated_top_level_keys() {
        let current = obj(&[
            (
                "statusLine",
                obj(&[("type", Value::String("command".into()))]),
            ),
            ("permissions", obj(&[("allow", Value::Array(vec![]))])),
            ("model", Value::String("old".into())),
        ]);
        let desired = DesiredSettings {
            model: Some("new".into()),
            ..Default::default()
        };
        let out = apply_desired(&current, &desired).expect("merge");
        assert_eq!(out["model"], "new");
        assert_eq!(out["statusLine"]["type"], "command");
        assert_eq!(out["permissions"]["allow"], Value::Array(vec![]));
    }

    #[test]
    fn merge_preserves_unrelated_env_keys() {
        let current = obj(&[(
            "env",
            obj(&[
                (
                    "ANTHROPIC_BASE_URL",
                    Value::String("http://127.0.0.1:31415".into()),
                ),
                (
                    "CLAUDE_CODE_AUTO_COMPACT_WINDOW",
                    Value::String("1048576".into()),
                ),
            ]),
        )]);
        let desired = DesiredSettings {
            env_api_key: Some("sk-ant-FAKE-111".into()),
            ..Default::default()
        };
        let out = apply_desired(&current, &desired).expect("merge");
        // Unmanaged env key untouched.
        assert_eq!(out["env"]["CLAUDE_CODE_AUTO_COMPACT_WINDOW"], "1048576");
        // Managed env key untouched because desired was None.
        assert_eq!(out["env"]["ANTHROPIC_BASE_URL"], "http://127.0.0.1:31415");
        assert_eq!(out["env"]["ANTHROPIC_API_KEY"], "sk-ant-FAKE-111");
    }

    #[test]
    fn none_means_untouched_not_removed() {
        let current = obj(&[("model", Value::String("keep-me".into()))]);
        let out = apply_desired(&current, &DesiredSettings::default()).expect("merge");
        assert_eq!(out["model"], "keep-me");
    }

    #[test]
    fn replaces_managed_env_key_value() {
        let current = obj(&[(
            "env",
            obj(&[("ANTHROPIC_AUTH_TOKEN", Value::String("old-token".into()))]),
        )]);
        let desired = DesiredSettings {
            env_auth_token: Some("new-token".into()),
            ..Default::default()
        };
        let out = apply_desired(&current, &desired).expect("merge");
        assert_eq!(out["env"]["ANTHROPIC_AUTH_TOKEN"], "new-token");
    }

    #[test]
    fn rejects_invalid_model_id() {
        let desired = DesiredSettings {
            model: Some("bad model\nid".into()),
            ..Default::default()
        };
        let err = apply_desired(&obj(&[]), &desired).expect_err("must reject");
        assert!(matches!(err, ConfigError::ValidationFailed(_)));
    }

    #[test]
    fn rejects_insecure_env_base_url() {
        let desired = DesiredSettings {
            env_base_url: Some("http://insecure.example.com".into()),
            ..Default::default()
        };
        let err = apply_desired(&obj(&[]), &desired).expect_err("must reject http");
        assert!(matches!(err, ConfigError::ValidationFailed(_)));
    }

    #[test]
    fn rejects_local_base_url_not_valid_https_remote() {
        // Local endpoints are allowed (Local classification), remote http is not.
        let desired = DesiredSettings {
            env_base_url: Some("https://localhost:8080".into()),
            ..Default::default()
        };
        assert!(apply_desired(&obj(&[]), &desired).is_ok());
    }

    #[test]
    fn rejects_non_object_current_document() {
        let err = apply_desired(&Value::Array(vec![]), &DesiredSettings::default())
            .expect_err("must reject array root");
        assert_eq!(err, ConfigError::MalformedShape("the current document"));
    }

    #[test]
    fn rejects_non_object_existing_env() {
        let current = obj(&[("env", Value::String("corrupted".into()))]);
        let desired = DesiredSettings {
            env_api_key: Some("sk-ant-FAKE".into()),
            ..Default::default()
        };
        let err = apply_desired(&current, &desired).expect_err("must refuse corrupted env");
        assert!(matches!(err, ConfigError::ValidationFailed(_)));
    }

    #[test]
    fn creates_env_when_missing_or_null() {
        let current = obj(&[("env", Value::Null)]);
        let desired = DesiredSettings {
            env_model: Some("provider-model".into()),
            ..Default::default()
        };
        let out = apply_desired(&current, &desired).expect("merge");
        assert_eq!(out["env"]["ANTHROPIC_MODEL"], "provider-model");
    }

    #[test]
    fn managed_key_registries_are_consistent() {
        // Every env key the DesiredSettings can write is in MANAGED_ENV_KEYS.
        let writable = [
            "ANTHROPIC_BASE_URL",
            "ANTHROPIC_AUTH_TOKEN",
            "ANTHROPIC_API_KEY",
            "ANTHROPIC_MODEL",
            "ANTHROPIC_DEFAULT_HAIKU_MODEL",
            "ANTHROPIC_DEFAULT_SONNET_MODEL",
            "ANTHROPIC_DEFAULT_OPUS_MODEL",
        ];
        assert_eq!(writable, MANAGED_ENV_KEYS);
    }

    #[test]
    fn helper_mode_writes_absolute_path() {
        let desired = DesiredSettings {
            api_key_helper: Some(
                "/Users/x/Library/Application Support/ClaudeConfigManager/helpers/anthropic.sh"
                    .into(),
            ),
            ..Default::default()
        };
        let out = apply_desired(&obj(&[]), &desired).expect("merge");
        assert_eq!(
            out["apiKeyHelper"],
            "/Users/x/Library/Application Support/ClaudeConfigManager/helpers/anthropic.sh"
        );
    }

    #[test]
    fn helper_mode_conflicts_with_plaintext_env_keys() {
        for conflicting in [
            DesiredSettings {
                api_key_helper: Some("/app/helpers/anthropic.sh".into()),
                env_api_key: Some("sk-ant-FAKE".into()),
                ..Default::default()
            },
            DesiredSettings {
                api_key_helper: Some("/app/helpers/anthropic.sh".into()),
                env_auth_token: Some("token-FAKE".into()),
                ..Default::default()
            },
        ] {
            let err = apply_desired(&obj(&[]), &conflicting).expect_err("must reject");
            assert!(matches!(err, ConfigError::ValidationFailed(_)));
        }
    }

    #[test]
    fn helper_path_must_be_absolute() {
        let desired = DesiredSettings {
            api_key_helper: Some("relative/helper.sh".into()),
            ..Default::default()
        };
        let err = apply_desired(&obj(&[]), &desired).expect_err("must reject relative");
        assert!(matches!(err, ConfigError::ValidationFailed(_)));
    }

    #[test]
    fn helper_path_must_reference_valid_provider_script() {
        let desired = DesiredSettings {
            api_key_helper: Some("/app/helpers/not a provider!.sh".into()),
            ..Default::default()
        };
        let err = apply_desired(&obj(&[]), &desired).expect_err("must reject bad stem");
        assert!(matches!(err, ConfigError::ValidationFailed(_)));
    }
}
