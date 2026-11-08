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

pub mod apply_lock;
pub mod config_backup;
pub mod config_locator;
pub mod config_merge;
pub mod config_reader;
pub mod config_restore;
pub mod config_status;
pub mod config_writer;
pub mod connection_test;
pub mod credential_handoff;
pub mod credential_index;
pub mod credential_migration;
pub mod credential_service;
pub mod credential_vault;
pub mod environment_check;
pub mod file_secret_store;
pub mod model_discovery;
pub mod model_discovery_service;
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
// The command surface is intentionally flat: Tauri deserializes each named
// argument from the webview, so grouping them would only move the field list.
#[allow(clippy::too_many_arguments)]
fn save_and_apply(
    provider_id: String,
    model: Option<String>,
    base_url: Option<String>,
    api_key: Option<String>,
    role_haiku: Option<String>,
    role_sonnet: Option<String>,
    role_opus: Option<String>,
    // V2-12b: OPTIONAL declared window for models Claude Code doesn't
    // recognize (it assumes 200k otherwise). `reset_context_window` removes
    // the managed key instead of writing a value.
    max_context_tokens: Option<u32>,
    reset_context_window: Option<bool>,
    // V2-21: which stored credential the helper must read. Absent keeps the
    // pre-V2 behaviour (`secrets/<provider>.secret`).
    credential_id: Option<String>,
) -> Result<save_apply::ApplyOutcome, CommandError> {
    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .map_err(|_| CommandError::not_available("HOME directory not determinable"))?;
    // Credentials live in an owner-only file (0600) under ~/.claude-config-manager
    // (a space-free path, because Claude Code does not quote apiKeyHelper).
    // The Keychain path was dropped: macOS refused it for the shipped bundle and
    // there was no way to sign around it (docs/audits/G13-file-credentials.md).
    let store: Arc<dyn secret_store::SecretStore> =
        Arc::new(file_secret_store::FileSecretStore::for_home(&home));
    let svc = save_apply::SaveApplyService::for_production(&home, store);
    // T45: role overrides are optional; absent = leave the managed env key
    // untouched (never invent a mapping).
    let roles = save_apply::RoleOverrides {
        haiku: role_haiku,
        sonnet: role_sonnet,
        opus: role_opus,
    };
    // The account must be a credential this install actually knows: either the
    // provider's legacy file or a record in the index. Anything else is refused
    // before a helper is written (a helper pointing at a missing file fails
    // closed at request time, which would look like "connection failed").
    let account = match credential_id {
        Some(id) => {
            credential_index::validate_credential_id(&id)
                .map_err(|_| CommandError::invalid_input("invalid credential id"))?;
            let vault = credential_vault::CredentialVault::for_home(&home);
            let known = vault
                .load()
                .map_err(|e| CommandError::io(e.to_string()))?
                .get(&id)
                .is_some();
            if !known {
                return Err(CommandError::invalid_input(
                    "no stored credential with that id — add it first",
                ));
            }
            Some(id)
        }
        None => None,
    };
    svc.apply_with_window(
        &provider_id,
        model.as_deref(),
        base_url.as_deref(),
        api_key.as_deref(),
        &roles,
        max_context_tokens,
        reset_context_window.unwrap_or(false),
        account.as_deref(),
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

/// "Test connection": prove the stored credential is actually accepted.
///
/// Save & Apply only proves the file was written; this performs one authenticated
/// request to a documented key-check endpoint and classifies the answer.
#[tauri::command]
fn test_connection(
    provider_id: String,
    credential: Option<String>,
) -> Result<connection_test::ConnectionTest, CommandError> {
    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .map_err(|_| CommandError::not_available("HOME directory not determinable"))?;
    let template = presets::load_builtin_presets()
        .into_iter()
        .find(|t| t.id == provider_id)
        .unwrap_or_else(|| {
            let mut t = presets::load_builtin_presets()
                .into_iter()
                .find(|p| p.id == "anthropic")
                .expect("anthropic preset");
            t.id = provider_id.clone();
            t.name = provider_id.clone();
            t.discovery = provider_catalog::DiscoverySpec::None;
            t
        });

    let store = file_secret_store::FileSecretStore::for_home(&home);
    let secret = match credential {
        Some(c) if !c.trim().is_empty() => c,
        _ => secret_store::SecretStore::get_secret(
            &store,
            &format!("com.claude-config-manager.{provider_id}"),
            &provider_id,
        )
        .map_err(|e| CommandError::io(e.to_string()))?
        .ok_or_else(|| {
            CommandError::not_available(
                "no credential stored for this provider — enter a key and save it first",
            )
        })?,
    };
    connection_test::test_connection(&template, secret.trim()).map_err(|e| match e {
        connection_test::TestError::Unsupported(_) => CommandError::not_available(e.to_string()),
        other => CommandError::io(other.to_string()),
    })
}

/// T38 contract: model discovery for one verified provider template.
#[tauri::command]
fn discover_models(
    provider_id: String,
    credential: Option<String>,
) -> Result<Vec<model_discovery::ModelEntry>, CommandError> {
    let store: Option<&dyn secret_store::SecretStore> = if credential.is_none() {
        // Same file-backed store the apply path writes to.
        Some(&file_secret_store::FileSecretStore::for_home(
            &std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default()),
        ))
    } else {
        None
    };
    model_discovery_service::discover_for_provider(&provider_id, credential.as_deref(), store)
        .map_err(|e| match e {
            model_discovery::DiscoveryError::InvalidEndpoint(m) => CommandError::invalid_input(m),
            other => CommandError::io(other.to_string()),
        })
}

/// V2-12a: does a credential already exist for this provider? The UI gates
/// "Save & Apply" on this so changing only the model (or roles) never forces
/// the owner to retype a key that is already stored at 0600.
#[tauri::command]
fn has_credential(provider_id: String) -> Result<bool, CommandError> {
    credential_handoff::validate_provider_id(&provider_id)
        .map_err(|_| CommandError::invalid_input("invalid provider id"))?;
    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .map_err(|_| CommandError::not_available("HOME directory not determinable"))?;
    let store = file_secret_store::FileSecretStore::for_home(&home);
    let creds = credential_service::CredentialService::new(&store);
    creds
        .has_credential(&provider_id)
        .map_err(|e| CommandError::io(e.to_string()))
}

/// V2-21: masked credential metadata for the UI. Never includes the key.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialView {
    pub id: String,
    pub provider_id: String,
    pub label: String,
    pub last4: String,
    pub priority: u32,
    pub enabled: bool,
    /// True for a migrated pre-V2 key: it lives in `secrets/<provider>.secret`
    /// and cannot be deleted from here without deleting that file, so the UI
    /// labels it and keeps it undeletable.
    pub legacy: bool,
}

impl CredentialView {
    fn from_record(r: &credential_index::CredentialRecord) -> Self {
        Self {
            id: r.id.clone(),
            provider_id: r.provider_id.clone(),
            label: r.label.clone(),
            last4: r.last4.clone(),
            priority: r.priority,
            enabled: r.enabled,
            legacy: r.id == credential_migration::legacy_credential_id(&r.provider_id),
        }
    }
}

fn vault() -> Result<credential_vault::CredentialVault, CommandError> {
    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .map_err(|_| CommandError::not_available("HOME directory not determinable"))?;
    Ok(credential_vault::CredentialVault::for_home(&home))
}

fn vault_error(e: credential_vault::VaultError) -> CommandError {
    match e {
        credential_vault::VaultError::Invalid(m) | credential_vault::VaultError::Index(
            credential_index::IndexError::Invalid(m),
        ) => CommandError::invalid_input(m),
        other => CommandError::io(other.to_string()),
    }
}

#[tauri::command]
fn list_credentials(provider_id: String) -> Result<Vec<CredentialView>, CommandError> {
    credential_handoff::validate_provider_id(&provider_id)
        .map_err(|_| CommandError::invalid_input("invalid provider id"))?;
    let list = vault()?.list(&provider_id).map_err(vault_error)?;
    Ok(list.iter().map(CredentialView::from_record).collect())
}

#[tauri::command]
fn add_credential(
    provider_id: String,
    label: String,
    secret: String,
) -> Result<CredentialView, CommandError> {
    let record = vault()?
        .add(&provider_id, &label, &secret)
        .map_err(vault_error)?;
    Ok(CredentialView::from_record(&record))
}

#[tauri::command]
fn remove_credential(credential_id: String) -> Result<bool, CommandError> {
    vault()?.remove(&credential_id).map_err(vault_error)
}

#[tauri::command]
fn set_credential_enabled(credential_id: String, enabled: bool) -> Result<bool, CommandError> {
    vault()?
        .set_enabled(&credential_id, enabled)
        .map_err(vault_error)
}

#[tauri::command]
fn promote_credential(credential_id: String) -> Result<bool, CommandError> {
    vault()?.promote(&credential_id).map_err(vault_error)
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
            check_environment,
            discover_models,
            test_connection,
            has_credential,
            list_credentials,
            add_credential,
            remove_credential,
            set_credential_enabled,
            promote_credential
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod contract_tests {
    use super::*;

    #[test]
    fn has_credential_rejects_an_unsafe_provider_id_before_touching_disk() {
        let err = has_credential("../../etc/passwd".into()).expect_err("must reject");
        assert!(matches!(err, CommandError::InvalidInput(_)));
    }

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
