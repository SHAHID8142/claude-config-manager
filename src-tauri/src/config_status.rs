//! Effective-configuration read-back with override warnings (T34, PRD F06).
//!
//! Reads the live settings file and reports what the app's managed keys are
//! currently set to, plus override/uncertainty warnings derived from the
//! T02 auth-precedence matrix:
//! - `apiKeyHelper` is SHADOWED when plaintext env credentials are also set
//!   (helper ranks below env keys).
//! - Plaintext credentials in `env` are flagged (disclosed tradeoff state).
//! - `model` vs `ANTHROPIC_MODEL` conflicts are flagged (env wins).

use crate::config_reader::{ConfigError, read_settings};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum StatusWarning {
    /// Helper credentials rank below env keys — the env value wins (T02).
    HelperShadowedByEnv,
    /// Plaintext credential material present in settings.json env.
    PlaintextKeyInEnv { which: String },
    /// `model` and `ANTHROPIC_MODEL` disagree; env wins per T02.
    ModelConflict {
        setting_model: String,
        env_model: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveStatus {
    pub settings_found: bool,
    pub model: Option<String>,
    pub env_model: Option<String>,
    /// Base URL currently in env (not a secret; disclosed to the user).
    pub env_base_url: Option<String>,
    pub auth_token_present: bool,
    pub api_key_present: bool,
    pub api_key_helper: Option<String>,
    /// Configured model-role overrides (T45). `None` = no override, which is
    /// the default: Claude Code's own role defaults apply.
    pub env_default_haiku: Option<String>,
    pub env_default_sonnet: Option<String>,
    pub env_default_opus: Option<String>,
    pub warnings: Vec<StatusWarning>,
}

/// Compute the effective status of the settings file at `path`.
/// Missing file → `settings_found: false`, empty state (first run).
pub fn read_effective_status(path: &Path) -> Result<EffectiveStatus, ConfigError> {
    let doc = match read_settings(path) {
        Ok(d) => d,
        Err(ConfigError::Missing) => {
            return Ok(EffectiveStatus {
                settings_found: false,
                model: None,
                env_model: None,
                env_base_url: None,
                auth_token_present: false,
                api_key_present: false,
                api_key_helper: None,
                env_default_haiku: None,
                env_default_sonnet: None,
                env_default_opus: None,
                warnings: Vec::new(),
            });
        }
        Err(e) => return Err(e),
    };

    let model = doc
        .get("model")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let env = doc.get("env").and_then(|v| v.as_object());

    let get_env = |key: &str| -> Option<String> {
        env.and_then(|m| m.get(key))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    };

    let env_model = get_env("ANTHROPIC_MODEL");
    let env_base_url = get_env("ANTHROPIC_BASE_URL");
    let env_default_haiku = get_env("ANTHROPIC_DEFAULT_HAIKU_MODEL");
    let env_default_sonnet = get_env("ANTHROPIC_DEFAULT_SONNET_MODEL");
    let env_default_opus = get_env("ANTHROPIC_DEFAULT_OPUS_MODEL");
    let auth_token_present = get_env("ANTHROPIC_AUTH_TOKEN").is_some();
    let api_key_present = get_env("ANTHROPIC_API_KEY").is_some();
    let api_key_helper = doc
        .get("apiKeyHelper")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let mut warnings = Vec::new();
    if api_key_helper.is_some() && (auth_token_present || api_key_present) {
        warnings.push(StatusWarning::HelperShadowedByEnv);
    }
    if api_key_present {
        warnings.push(StatusWarning::PlaintextKeyInEnv {
            which: "ANTHROPIC_API_KEY".into(),
        });
    }
    if auth_token_present {
        warnings.push(StatusWarning::PlaintextKeyInEnv {
            which: "ANTHROPIC_AUTH_TOKEN".into(),
        });
    }
    if let (Some(setting), Some(envm)) = (&model, &env_model)
        && setting != envm
    {
        warnings.push(StatusWarning::ModelConflict {
            setting_model: setting.clone(),
            env_model: envm.clone(),
        });
    }

    Ok(EffectiveStatus {
        settings_found: true,
        model,
        env_model,
        env_base_url,
        auth_token_present,
        api_key_present,
        api_key_helper,
        env_default_haiku,
        env_default_sonnet,
        env_default_opus,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ccm-status-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("dir");
        dir
    }

    #[test]
    fn role_overrides_are_reported_and_default_to_none() {
        // T45: the advanced panel must be able to show what is really set,
        // so "no override" is never misreported as an override.
        let dir = temp("roles");
        let path = dir.join("settings.json");
        fs::write(
            &path,
            r#"{"env":{"ANTHROPIC_DEFAULT_SONNET_MODEL":"main-model"}}"#,
        )
        .expect("seed");

        let status = read_effective_status(&path).expect("status");
        assert_eq!(status.env_default_sonnet.as_deref(), Some("main-model"));
        assert_eq!(status.env_default_haiku, None);
        assert_eq!(status.env_default_opus, None);

        let missing = read_effective_status(&dir.join("absent.json")).expect("missing ok");
        assert!(!missing.settings_found);
        assert_eq!(missing.env_default_haiku, None);
        assert_eq!(missing.env_default_sonnet, None);
        assert_eq!(missing.env_default_opus, None);
        fs::remove_dir_all(&dir).ok();
    }

    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ccm-t34-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    fn status_of(json: &str) -> EffectiveStatus {
        let dir = temp_dir("probe");
        let path = dir.join("settings.json");
        fs::write(&path, json).expect("write fixture");
        let s = read_effective_status(&path).expect("status");
        fs::remove_dir_all(&dir).expect("cleanup");
        s
    }

    #[test]
    fn missing_file_reports_first_run_state() {
        let dir = temp_dir("missing");
        let s = read_effective_status(&dir.join("settings.json")).expect("status");
        assert!(!s.settings_found);
        assert!(s.warnings.is_empty());
        fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn helper_only_configuration_is_clean() {
        let s = status_of(r#"{"apiKeyHelper":"/app/helpers/anthropic.sh"}"#);
        assert!(s.settings_found);
        assert_eq!(
            s.api_key_helper.as_deref(),
            Some("/app/helpers/anthropic.sh")
        );
        assert!(!s.auth_token_present);
        assert!(!s.api_key_present);
        assert!(s.warnings.is_empty(), "no warnings expected: {s:?}");
    }

    #[test]
    fn helper_shadowed_by_plaintext_env_is_flagged() {
        let s = status_of(
            r#"{
                "apiKeyHelper": "/app/helpers/anthropic.sh",
                "env": {"ANTHROPIC_API_KEY": "sk-ant-FAKE"}
            }"#,
        );
        assert!(
            s.warnings
                .iter()
                .any(|w| *w == StatusWarning::HelperShadowedByEnv),
            "shadowing must be flagged: {s:?}"
        );
        // Plaintext presence flagged too — with the key NAME, never the value.
        assert!(s.warnings.iter().any(
            |w| matches!(w, StatusWarning::PlaintextKeyInEnv { which } if which == "ANTHROPIC_API_KEY")
        ));
    }

    #[test]
    fn plaintext_keys_flagged_without_helper() {
        let s = status_of(
            r#"{"env":{"ANTHROPIC_AUTH_TOKEN":"tok","ANTHROPIC_BASE_URL":"http://127.0.0.1:31415"}}"#,
        );
        assert!(s.auth_token_present);
        assert_eq!(
            s.env_base_url.as_deref(),
            Some("http://127.0.0.1:31415"),
            "base URL disclosed (not a secret)"
        );
        assert_eq!(s.warnings.len(), 1);
        assert!(matches!(
            &s.warnings[0],
            StatusWarning::PlaintextKeyInEnv { which } if which == "ANTHROPIC_AUTH_TOKEN"
        ));
    }

    #[test]
    fn model_conflict_flagged_env_wins() {
        let s = status_of(r#"{"model":"claude-x","env":{"ANTHROPIC_MODEL":"claude-y"}}"#);
        assert!(s.warnings.iter().any(
            |w| matches!(w, StatusWarning::ModelConflict { setting_model, env_model }
                if setting_model == "claude-x" && env_model == "claude-y")
        ));
    }

    #[test]
    fn no_conflict_when_models_agree() {
        let s = status_of(r#"{"model":"claude-x","env":{"ANTHROPIC_MODEL":"claude-x"}}"#);
        assert!(s.warnings.is_empty());
    }

    #[test]
    fn malformed_file_surfaces_error_not_zero_status() {
        let dir = temp_dir("malformed");
        let path = dir.join("settings.json");
        fs::write(&path, "{broken").expect("bad json");
        assert!(matches!(
            read_effective_status(&path),
            Err(ConfigError::MalformedJson(_))
        ));
        fs::remove_dir_all(&dir).expect("cleanup");
    }
}
