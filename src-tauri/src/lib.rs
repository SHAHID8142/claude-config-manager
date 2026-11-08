//! Typed command-contract primitives shared by all Tauri commands.
//!
//! Contract rules (docs/architecture/MODULE-BOUNDARIES.md):
//! - Commands return `Result<T, CommandError>` so the UI always receives a
//!   structured, redaction-safe error instead of a raw panic string.
//! - `CommandError` messages are plain, user-mappable strings — handlers must
//!   pre-redact any secret-shaped content before constructing one (T23 adds
//!   the shared redaction guard).

use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub mod config_backup;
pub mod config_locator;
pub mod config_merge;
pub mod config_reader;
pub mod config_restore;
pub mod config_status;
pub mod config_writer;
pub mod credential_handoff;
pub mod credential_service;
pub mod environment_check;
pub mod keychain_store;
pub mod model_discovery;
pub mod presets;
pub mod provider_catalog;
pub mod redact;
pub mod save_apply;
pub mod secret_store;

/// Structured error payload crossing the UI boundary (T11 contract).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "message", rename_all = "snake_case")]
pub enum CommandError {
    /// Caller-supplied input failed validation (field name included).
    InvalidInput(String),
    /// Filesystem or OS interaction failed; message carries no secret material.
    Io(String),
    /// Capability not available in the current build/environment.
    NotAvailable(String),
}

impl CommandError {
    pub fn invalid_input(message: impl Into<String>) -> Self {
        CommandError::InvalidInput(message.into())
    }

    pub fn io(message: impl Into<String>) -> Self {
        CommandError::Io(message.into())
    }

    pub fn not_available(message: impl Into<String>) -> Self {
        CommandError::NotAvailable(message.into())
    }
}

#[tauri::command]
fn greet(name: &str) -> Result<String, CommandError> {
    if name.len() > 256 {
        // Negative guard: oversized names rejected with a typed error.
        return Err(CommandError::invalid_input("name exceeds 256 characters"));
    }
    Ok(format!("Hello, {}! You've been greeted from Rust!", name))
}

/// T18 contract: list bundled provider templates for the selector.
#[tauri::command]
fn list_providers() -> Result<Vec<provider_catalog::ProviderSummary>, CommandError> {
    Ok(presets::load_builtin_presets()
        .iter()
        .map(provider_catalog::ProviderSummary::from_template)
        .collect())
}

/// T33 contract: full Save & Apply pipeline for one verified direct provider.
#[tauri::command]
fn save_and_apply(
    provider_id: String,
    model: Option<String>,
    base_url: Option<String>,
    api_key: Option<String>,
) -> Result<save_apply::ApplyOutcome, CommandError> {
    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .map_err(|_| CommandError::not_available("HOME directory not determinable"))?;
    let store: Arc<dyn secret_store::SecretStore> = Arc::new(keychain_store::KeychainStore::new());
    let svc = save_apply::SaveApplyService::for_production(&home, store);
    svc.apply(
        &provider_id,
        model.as_deref(),
        base_url.as_deref(),
        api_key.as_deref(),
    )
    .map_err(|e| match e {
        save_apply::ApplyError::InvalidInput(m) => CommandError::invalid_input(m),
        save_apply::ApplyError::NoCredential => CommandError::invalid_input(e.to_string()),
        other => CommandError::io(other.to_string()),
    })
}

/// T34 contract: read-back effective status with override warnings.
#[tauri::command]
fn read_status() -> Result<config_status::EffectiveStatus, CommandError> {
    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .map_err(|_| CommandError::not_available("HOME directory not determinable"))?;
    let path = config_locator::ConfigLocator::new().locate(&home, None);
    config_status::read_effective_status(&path).map_err(|e| CommandError::io(e.to_string()))
}

/// T35 contract: restore the newest settings backup (with pre-restore copy).
#[tauri::command]
fn restore_last_backup() -> Result<config_restore::RestoreOutcome, CommandError> {
    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .map_err(|_| CommandError::not_available("HOME directory not determinable"))?;
    let settings = config_locator::ConfigLocator::new().locate(&home, None);
    let backups = config_backup::default_backup_dir(&home);
    config_restore::restore_last_backup(&settings, &backups)
        .map_err(|e| CommandError::io(e.to_string()))
}

/// T36 contract: environment diagnostics (CLI presence/version, settings state).
#[tauri::command]
fn check_environment() -> Result<environment_check::EnvironmentReport, CommandError> {
    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .map_err(|_| CommandError::not_available("HOME directory not determinable"))?;
    Ok(environment_check::check_environment(&home, None))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            greet,
            list_providers,
            save_and_apply,
            read_status,
            restore_last_backup,
            check_environment
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod contract_tests {
    use super::*;

    #[test]
    fn command_error_serializes_with_kind_and_message() {
        let err = CommandError::invalid_input("endpoint must be https");
        let json = serde_json::to_value(&err).expect("serialize");
        assert_eq!(json["kind"], "invalid_input");
        assert_eq!(json["message"], "endpoint must be https");
    }

    #[test]
    fn command_error_kind_names_are_snake_case() {
        for (err, expected) in [
            (CommandError::io("read failed"), "io"),
            (CommandError::not_available("x"), "not_available"),
            (CommandError::invalid_input("y"), "invalid_input"),
        ] {
            let json = serde_json::to_value(&err).expect("serialize");
            assert_eq!(json["kind"], expected);
        }
    }

    #[test]
    fn greet_rejects_oversized_name_with_typed_error() {
        let long = "a".repeat(257);
        assert!(matches!(greet(&long), Err(CommandError::InvalidInput(_))));
    }

    #[test]
    fn greet_accepts_normal_name() {
        let out = greet("fixture-user").expect("ok");
        assert!(out.contains("fixture-user"));
    }

    #[test]
    fn list_providers_returns_bundled_templates() {
        let providers = list_providers().expect("ok");
        assert!(!providers.is_empty());
        assert_eq!(providers[0].id, "anthropic");
        // Wire shape check: camelCase keys for the TS contract.
        let json = serde_json::to_value(&providers[0]).expect("serialize");
        assert!(json.get("capabilityStatus").is_some());
        assert!(json.get("docsUrl").is_some());
    }
}
