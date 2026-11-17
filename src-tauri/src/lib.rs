//! Typed command-contract primitives shared by all Tauri commands.
//!
//! Contract rules (docs/architecture/MODULE-BOUNDARIES.md):
//! - Commands return `Result<T, CommandError>` so the UI always receives a
//!   structured, redaction-safe error instead of a raw panic string.
//! - `CommandError` messages are plain, user-mappable strings — handlers must
//!   pre-redact any secret-shaped content before constructing one (T23 adds
//!   the shared redaction guard).

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

pub mod apply_lock;
pub mod bridge;
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
pub mod failover;
pub mod file_secret_store;
pub mod helper;
pub mod model_discovery;
pub mod model_discovery_service;
pub mod nvidia_adapter;
pub mod nvidia_setup;
pub mod presets;
pub mod provider_catalog;
pub mod proxy;
pub mod redact;
pub mod routing_config;
pub mod save_apply;
pub mod secret_store;
pub mod switch_history;
pub mod transaction_journal;

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

/// T18 contract: list bundled provider templates for the selector.
#[tauri::command(async)]
fn list_providers() -> Result<Vec<provider_catalog::ProviderSummary>, CommandError> {
    Ok(presets::load_builtin_presets()
        .iter()
        .map(provider_catalog::ProviderSummary::from_template)
        .collect())
}

/// T33 contract: full Save & Apply pipeline for one verified direct provider.
#[tauri::command(async)]
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
    reset_roles: Option<Vec<String>>,
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
    // (the generated command safely quotes paths for Claude Code's shell).
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
        reset: ["haiku", "sonnet", "opus"].map(|role| {
            reset_roles
                .as_ref()
                .is_some_and(|roles| roles.iter().any(|r| r == role))
        }),
    };
    // The account must be a credential this install actually knows: either the
    // provider's legacy file or a record in the index. Anything else is refused
    // before a helper is written (a helper pointing at a missing file fails
    // closed at request time, which would look like "connection failed").
    if let Some(template) = presets::load_builtin_presets()
        .iter()
        .find(|p| p.id == provider_id)
    {
        if template.protocol == provider_catalog::Protocol::OpenAiOnly
            && provider_id != "nvidia-nim"
        {
            return Err(CommandError::not_available(
                "This provider requires a protocol bridge, which is not available in this release.",
            ));
        }
    } else if provider_id != "custom" && !provider_id.starts_with("custom-") {
        return Err(CommandError::invalid_input("unknown provider"));
    }
    let records = credential_vault::CredentialVault::for_home(&home)
        .list(&provider_id)
        .map_err(vault_error)?;
    let account = match credential_id {
        Some(id) => {
            if !records
                .iter()
                .any(|c| c.id == id && c.provider_id == provider_id && c.enabled)
            {
                return Err(CommandError::invalid_input(
                    "choose an enabled key belonging to this provider",
                ));
            }
            Some(id)
        }
        None => records.iter().find(|c| c.enabled).map(|c| c.id.clone()),
    };
    if api_key.as_deref().is_none_or(|s| s.trim().is_empty()) && account.is_none() {
        return Err(CommandError::invalid_input(
            "add or enable a key for this provider",
        ));
    }
    if provider_id == "nvidia-nim" {
        let model = model
            .as_deref()
            .filter(|m| !m.trim().is_empty())
            .ok_or_else(|| {
                CommandError::invalid_input("Choose a NVIDIA model before Save & Apply")
            })?;
        let previous = helper::running_identity(&home);
        let result = nvidia_setup::apply(
            &svc,
            model,
            api_key.as_deref(),
            &roles,
            max_context_tokens,
            reset_context_window.unwrap_or(false),
            || {
                helper::stop(&home)?;
                helper::start(&home).map(|state| state.port)
            },
        );
        if result.is_err()
            && matches!(transaction_journal::pending(&home), Ok(None))
            && let Some(previous) = previous
            && let Err(error) = helper::restore_running(&home, &previous)
        {
            return Err(CommandError::io(format!(
                "NVIDIA activation failed; previous helper could not be restarted ({error}). Open Recovery & diagnostics."
            )));
        }
        return result.map_err(|e| CommandError::io(e.to_string()));
    }
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProxyInfo {
    port: u16,
    helper_path: String,
}

fn home_dir() -> Result<std::path::PathBuf, CommandError> {
    std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .map_err(|_| CommandError::not_available("HOME directory not determinable"))
}

/// V2-23: read the ordered routing chain (metadata only, no secrets).
#[tauri::command(async)]
fn get_routing() -> Result<routing_config::RoutingConfig, CommandError> {
    let home = home_dir()?;
    routing_config::load(&home).map_err(|e| CommandError::io(e.to_string()))
}

/// V2-23: replace the routing chain. Validation fails closed: nothing is
/// written unless the whole chain is accepted.
#[tauri::command(async)]
fn save_routing(config: routing_config::RoutingConfig) -> Result<(), CommandError> {
    let home = home_dir()?;
    config
        .validate()
        .map_err(|e| CommandError::invalid_input(e.to_string()))?;
    if config.targets.iter().any(|t| !t.fallback_models.is_empty())
        && helper::running_identity(&home).is_some()
    {
        helper::start(&home).map_err(CommandError::io)?;
    }
    routing_config::save(&home, &config).map_err(|e| CommandError::io(e.to_string()))
}

/// V3-05: start the detached proxy helper. Idempotent: a verified helper that
/// is already running is returned unchanged.
#[tauri::command(async)]
fn proxy_start() -> Result<ProxyInfo, CommandError> {
    let home = home_dir()?;
    let st = helper::start(&home).map_err(CommandError::io)?;
    Ok(ProxyInfo {
        port: st.port,
        helper_path: credential_handoff::default_helpers_dir(&home)
            .join("proxy.sh")
            .to_string_lossy()
            .to_string(),
    })
}

/// Stop the helper. Only a pid that answers on its own port is signalled.
#[tauri::command(async)]
fn proxy_stop() -> Result<bool, CommandError> {
    let home = home_dir()?;
    helper::stop(&home).map_err(CommandError::io)
}

/// V3-26: where things actually live, so the UI never invents a path.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfigPaths {
    home: String,
    /// `~/.claude-config-manager/secrets` (0600 files, 0700 dir).
    secrets_dir: String,
    /// `~/.claude-config-manager/helpers`; command paths are shell-quoted safely.
    helpers_dir: String,
    /// `~/.claude-config-manager/routing.json`
    routing_file: String,
    /// The Claude Code settings file this app writes.
    settings_file: String,
    /// `~/.claude-config-manager/proxy-helper.json`
    helper_state: String,
}

#[tauri::command(async)]
fn config_paths() -> Result<ConfigPaths, CommandError> {
    let home = home_dir()?;
    let settings = config_locator::ConfigLocator::new()
        .locate(&home, config_locator::production_config_dir().as_deref());
    Ok(ConfigPaths {
        secrets_dir: home
            .join(".claude-config-manager")
            .join("secrets")
            .to_string_lossy()
            .to_string(),
        helpers_dir: credential_handoff::default_helpers_dir(&home)
            .to_string_lossy()
            .to_string(),
        routing_file: routing_config::routing_path(&home)
            .to_string_lossy()
            .to_string(),
        settings_file: settings.to_string_lossy().to_string(),
        helper_state: helper::state_path(&home).to_string_lossy().to_string(),
        home: home.to_string_lossy().to_string(),
    })
}

/// V3-07: restart (used when the helper is stale or the app was updated).
#[tauri::command(async)]
fn proxy_restart() -> Result<ProxyInfo, CommandError> {
    let home = home_dir()?;
    let st = helper::restart(&home).map_err(CommandError::io)?;
    Ok(ProxyInfo {
        port: st.port,
        helper_path: credential_handoff::default_helpers_dir(&home)
            .join("proxy.sh")
            .to_string_lossy()
            .to_string(),
    })
}

/// V2-30 counters, now read from the helper's `/healthz` instead of memory.
#[tauri::command(async)]
fn proxy_status() -> Result<Option<proxy::ProxyStatus>, CommandError> {
    let home = home_dir()?;
    let st = helper::status(&home);
    if !st.running {
        return Ok(None);
    }
    let port = st
        .port
        .ok_or_else(|| CommandError::io("helper has no port"))?;
    Ok(helper::health(port))
}

/// V3-09: whether routed mode survives a login (explicit opt-in only).
#[tauri::command(async)]
fn helper_status() -> Result<helper::HelperStatus, CommandError> {
    Ok(helper::status(&home_dir()?))
}

/// V3-09: install (`true`) or remove (`false`) the user LaunchAgent.
///
/// This is the only persistent-artifact action in the app, it is never called
/// implicitly, and it writes only under `~/Library/LaunchAgents` plus the
/// owner's own `~/.claude-config-manager` directory.
#[tauri::command(async)]
fn helper_login_start(enabled: bool) -> Result<helper::HelperStatus, CommandError> {
    let home = home_dir()?;
    if enabled {
        helper::install_login_agent(&home).map_err(CommandError::io)?;
    } else {
        helper::uninstall_login_agent(&home).map_err(CommandError::io)?;
    }
    Ok(helper::status(&home))
}

/// V2-32: one health result per routing target.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TargetHealth {
    target_id: String,
    provider_id: String,
    ok: bool,
    /// Human sentence; never contains the credential.
    message: String,
}

/// V2-32: check every routing target on demand.
///
/// This uses the provider's documented key-check endpoint and never sends a
/// completion request, so checking the chain costs nothing and cannot consume
/// quota. Targets without a stored key, or providers without a documented
/// key-check endpoint, report `ok: false` with an honest reason.
#[tauri::command(async)]
fn check_routing_health() -> Result<Vec<TargetHealth>, CommandError> {
    let home = home_dir()?;
    let cfg = routing_config::load(&home).map_err(|e| CommandError::io(e.to_string()))?;
    let templates = presets::load_builtin_presets();
    let mut out = Vec::new();
    let expanded = proxy::expand_provider_keys(&home, &cfg);
    for target in expanded.ordered() {
        if !target.enabled {
            continue;
        }
        let Some(template) = templates.iter().find(|t| t.id == target.provider_id) else {
            out.push(TargetHealth {
                target_id: target.id.clone(),
                provider_id: target.provider_id.clone(),
                ok: false,
                message: "this provider is not in the bundled catalog".into(),
            });
            continue;
        };
        let Some(secret) = proxy::resolve_secret(&home, target) else {
            out.push(TargetHealth {
                target_id: target.id.clone(),
                provider_id: target.provider_id.clone(),
                ok: false,
                message: "no stored key for this target".into(),
            });
            continue;
        };
        let (ok, message) = match connection_test::test_connection(template, &secret) {
            Ok(result) => (result.ok, result.message),
            Err(e) => (false, e.to_string()),
        };
        out.push(TargetHealth {
            target_id: target.id.clone(),
            provider_id: target.provider_id.clone(),
            ok,
            message,
        });
    }
    Ok(out)
}

/// V2-31: point Claude Code at the proxy (base URL + token helper).
#[tauri::command(async)]
fn apply_routed_mode(
    model: Option<String>,
    config: Option<routing_config::RoutingConfig>,
) -> Result<save_apply::ApplyOutcome, CommandError> {
    let home = home_dir()?;
    let cfg = match config {
        Some(cfg) => cfg,
        None => routing_config::load(&home).map_err(|e| CommandError::io(e.to_string()))?,
    };
    let store = std::sync::Arc::new(file_secret_store::FileSecretStore::for_home(&home));
    let svc = save_apply::SaveApplyService::for_production(&home, store);
    svc.apply_routing_policy(&cfg, model.as_deref(), || {
        helper::start(&home).map(|state| state.port)
    })
    .map_err(|e| CommandError::io(e.to_string()))
}

#[tauri::command(async)]
fn switch_history() -> Result<Vec<switch_history::Event>, CommandError> {
    switch_history::read(&home_dir()?).map_err(CommandError::io)
}

/// T34 contract: read-back effective status with override warnings.
#[tauri::command(async)]
fn read_status() -> Result<config_status::EffectiveStatus, CommandError> {
    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .map_err(|_| CommandError::not_available("HOME directory not determinable"))?;
    let path = config_locator::ConfigLocator::new()
        .locate(&home, config_locator::production_config_dir().as_deref());
    config_status::read_effective_status(&path).map_err(|e| CommandError::io(e.to_string()))
}

/// T35 contract: restore the newest settings backup (with pre-restore copy).
#[tauri::command(async)]
fn restore_last_backup(
    file_name: Option<String>,
) -> Result<config_restore::RestoreOutcome, CommandError> {
    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .map_err(|_| CommandError::not_available("HOME directory not determinable"))?;
    let settings = config_locator::ConfigLocator::new()
        .locate(&home, config_locator::production_config_dir().as_deref());
    let backups = config_backup::default_backup_dir(&home);
    config_restore::restore_backup(&settings, &backups, file_name.as_deref())
        .map_err(|e| CommandError::io(e.to_string()))
}

/// Explicit recovery: invalid settings are archived before any replacement.
#[tauri::command(async)]
fn recover_settings(
    use_backup: bool,
    file_name: Option<String>,
) -> Result<config_restore::RestoreOutcome, CommandError> {
    let home = home_dir()?;
    let settings = config_locator::ConfigLocator::new()
        .locate(&home, config_locator::production_config_dir().as_deref());
    config_restore::recover_settings_selected(
        &settings,
        &config_backup::default_backup_dir(&home),
        use_backup,
        file_name.as_deref(),
    )
    .map_err(|e| CommandError::io(e.to_string()))
}

#[tauri::command(async)]
fn pending_transaction() -> Result<Option<transaction_journal::PendingSummary>, CommandError> {
    transaction_journal::pending(&home_dir()?).map_err(CommandError::io)
}
#[tauri::command(async)]
fn recover_transaction(
    restore_previous: bool,
) -> Result<transaction_journal::RecoveryOutcome, CommandError> {
    let home = home_dir()?;
    let settings = config_locator::ConfigLocator::new()
        .locate(&home, config_locator::production_config_dir().as_deref());
    transaction_journal::recover(&home, Some(&settings), restore_previous).map_err(CommandError::io)
}

/// D9 (Maintenance): list available settings backups, newest first —
/// metadata only, never content.
#[tauri::command(async)]
fn list_backups() -> Result<Vec<config_backup::BackupInfo>, CommandError> {
    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .map_err(|_| CommandError::not_available("HOME directory not determinable"))?;
    let backups = config_backup::default_backup_dir(&home);
    config_backup::list_backups(&backups).map_err(|e| CommandError::io(e.to_string()))
}

#[tauri::command(async)]
fn backup_previews() -> Result<Vec<config_restore::BackupPreview>, CommandError> {
    let home = home_dir()?;
    config_restore::list_backup_previews(&config_backup::default_backup_dir(&home))
        .map_err(|e| CommandError::io(e.to_string()))
}

/// T36 contract: environment diagnostics (CLI presence/version, settings state).
#[tauri::command(async)]
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
#[tauri::command(async)]
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

    let secret = match credential {
        Some(c) if !c.trim().is_empty() => c,
        _ => credential_vault::CredentialVault::for_home(&home)
            .resolve(&provider_id, None)
            .map_err(vault_error)?
            .ok_or_else(|| CommandError::not_available("add or enable a key for this provider"))?,
    };
    connection_test::test_connection(&template, secret.trim()).map_err(|e| match e {
        connection_test::TestError::Unsupported(_) => CommandError::not_available(e.to_string()),
        other => CommandError::io(other.to_string()),
    })
}

/// T38 contract: model discovery for one verified provider template.
#[tauri::command(async)]
fn discover_models(
    provider_id: String,
    credential: Option<String>,
) -> Result<Vec<model_discovery::ModelEntry>, CommandError> {
    let requires_auth = presets::load_builtin_presets()
        .iter()
        .find(|p| p.id == provider_id)
        .is_some_and(|p| {
            matches!(
                p.discovery,
                provider_catalog::DiscoverySpec::Endpoint {
                    requires_auth: true,
                    ..
                }
            )
        });
    let resolved = if !requires_auth {
        None
    } else if credential.as_deref().is_none_or(|s| s.trim().is_empty()) {
        vault()?.resolve(&provider_id, None).map_err(vault_error)?
    } else {
        credential
    };
    model_discovery_service::discover_for_provider(&provider_id, resolved.as_deref(), None).map_err(
        |e| match e {
            model_discovery::DiscoveryError::InvalidEndpoint(m) => CommandError::invalid_input(m),
            other => CommandError::io(other.to_string()),
        },
    )
}

/// V2-12a: does a credential already exist for this provider? The UI gates
/// "Save & Apply" on this so changing only the model (or roles) never forces
/// the owner to retype a key that is already stored at 0600.
#[tauri::command(async)]
fn has_credential(provider_id: String) -> Result<bool, CommandError> {
    credential_handoff::validate_provider_id(&provider_id)
        .map_err(|_| CommandError::invalid_input("invalid provider id"))?;
    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .map_err(|_| CommandError::not_available("HOME directory not determinable"))?;
    // Vault-aware: a provider whose keys live only as indexed credentials
    // (secrets/<provider>-N.secret) must answer true, otherwise the UI shows
    // the "add a key to enable Save & Apply" hint next to a listed key.
    let vault = credential_vault::CredentialVault::for_home(&home);
    vault.has_any(&provider_id).map_err(vault_error)
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
        credential_vault::VaultError::Invalid(m)
        | credential_vault::VaultError::Index(credential_index::IndexError::Invalid(m)) => {
            CommandError::invalid_input(m)
        }
        other => CommandError::io(other.to_string()),
    }
}

#[tauri::command(async)]
fn list_credentials(provider_id: String) -> Result<Vec<CredentialView>, CommandError> {
    credential_handoff::validate_provider_id(&provider_id)
        .map_err(|_| CommandError::invalid_input("invalid provider id"))?;
    let list = vault()?.list(&provider_id).map_err(vault_error)?;
    Ok(list.iter().map(CredentialView::from_record).collect())
}

#[tauri::command(async)]
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

fn protect_active_direct_key(credential_id: &str) -> Result<(), CommandError> {
    let home = home_dir()?;
    let vault = credential_vault::CredentialVault::for_home(&home);
    let index = vault.load().map_err(vault_error)?;
    let Some(record) = index.get(credential_id) else {
        return Ok(());
    };
    let account =
        if credential_id == credential_migration::legacy_credential_id(&record.provider_id) {
            &record.provider_id
        } else {
            credential_id
        };
    let path = config_locator::ConfigLocator::new()
        .locate(&home, config_locator::production_config_dir().as_deref());
    if let Ok(status) = config_status::read_effective_status(&path)
        && let Some(helper) = status.api_key_helper
        && let Some(helper_path) = credential_handoff::helper_command_path(&helper)
        && helper_path.parent() == Some(credential_handoff::default_helpers_dir(&home).as_path())
        && let Ok(script) = std::fs::read_to_string(&helper_path)
        && script.contains(&format!("secrets/{account}.secret"))
    {
        return Err(CommandError::invalid_input(
            "This key is used by the saved direct configuration. Apply another key or enable automatic switching before disabling or deleting it.",
        ));
    }
    Ok(())
}

#[tauri::command(async)]
fn remove_credential(credential_id: String) -> Result<bool, CommandError> {
    protect_active_direct_key(&credential_id)?;
    vault()?.remove(&credential_id).map_err(vault_error)
}

#[tauri::command(async)]
fn set_credential_enabled(credential_id: String, enabled: bool) -> Result<bool, CommandError> {
    if !enabled {
        protect_active_direct_key(&credential_id)?;
    }
    vault()?
        .set_enabled(&credential_id, enabled)
        .map_err(vault_error)
}

#[tauri::command(async)]
fn promote_credential(credential_id: String) -> Result<bool, CommandError> {
    vault()?.promote(&credential_id).map_err(vault_error)
}

// ─── Bridge (T48/T49/T52/T54/T58) — consent-gated third-party translator ──

fn bridge_state_dir(home: &Path) -> std::path::PathBuf {
    home.join("Library")
        .join("Application Support")
        .join("ClaudeConfigManager")
        .join("bridge")
}

fn bridge_install_root(home: &Path) -> std::path::PathBuf {
    bridge_state_dir(home).join("packages")
}

/// T48: the consent payload the UI displays before asking. Facts only —
/// everything here is from the committed pin, nothing is fetched.
#[tauri::command(async)]
fn bridge_consent_info() -> bridge::manifest::BridgePin {
    // Return an owned copy — Tauri commands serialize owned values.
    bridge::manifest::PINNED.clone()
}

/// T48: record explicit consent (version-stamped). The installer and the
/// runtime both refuse without this marker — enforced server-side, not in
/// the UI.
#[tauri::command(async)]
fn bridge_consent_record() -> Result<(), CommandError> {
    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .map_err(|_| CommandError::not_available("HOME directory not determinable"))?;
    bridge::runtime::write_consent_marker(&bridge_state_dir(&home))
        .map(|_| ())
        .map_err(|e| CommandError::io(e.to_string()))
}

/// T48: withdraw consent (also stops a running bridge first).
#[tauri::command(async)]
fn bridge_consent_withdraw() -> Result<(), CommandError> {
    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .map_err(|_| CommandError::not_available("HOME directory not determinable"))?;
    let state = bridge_state_dir(&home);
    let _ = bridge::runtime::stop(&state);
    bridge::runtime::clear_consent_marker(&state);
    Ok(())
}

/// T52 status: installed? consented? running?
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct BridgeOverview {
    consented: bool,
    installed: bool,
    version: String,
    node_found: bool,
    node_major: Option<u32>,
    running: bool,
    pid: Option<u32>,
    gateway_port: u16,
    /// Honest feature gate: the pinned ccr 3.1.3 keeps providers in its own
    /// SQLite store, and this build never writes provider configuration into
    /// it — an installed, running gateway has nothing to route to. The UI
    /// surfaces this instead of offering a bridge that cannot serve requests.
    provider_config_ready: bool,
}

#[tauri::command(async)]
fn bridge_overview() -> Result<BridgeOverview, CommandError> {
    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .map_err(|_| CommandError::not_available("HOME directory not determinable"))?;
    let state = bridge_state_dir(&home);
    let st = bridge::runtime::status(&state);
    let installed = bridge::installer::version_dir(
        &bridge_install_root(&home),
        bridge::manifest::PINNED.version,
    )
    .join("dist/main/cli.js")
    .is_file();
    let node =
        bridge::installer::detect_node_with_home(&std::env::var("PATH").unwrap_or_default(), &home);
    Ok(BridgeOverview {
        consented: bridge::runtime::consented_marker(&state),
        installed,
        version: bridge::manifest::PINNED.version.to_string(),
        node_found: node.is_some(),
        node_major: node.map(|(_, m)| m),
        running: st.running,
        pid: st.pid,
        gateway_port: st.gateway_port,
        provider_config_ready: false,
    })
}

/// T49/T50: install (download → integrity → extract). Requires consent.
#[tauri::command(async)]
fn bridge_install() -> Result<bridge::installer::InstallReport, CommandError> {
    Err(CommandError::not_available(
        "The protocol bridge is disabled in this release. Use an Anthropic-compatible provider; no bridge download or process was started.",
    ))
}

/// T52: start the detached bridge (requires consent + install).
#[tauri::command(async)]
fn bridge_start() -> Result<u32, CommandError> {
    Err(CommandError::not_available(
        "The protocol bridge is disabled in this release. Use an Anthropic-compatible provider; no bridge download or process was started.",
    ))
}

/// T52: stop the bridge.
#[tauri::command(async)]
fn bridge_stop() -> Result<bool, CommandError> {
    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .map_err(|_| CommandError::not_available("HOME directory not determinable"))?;
    bridge::runtime::stop(&bridge_state_dir(&home)).map_err(|e| CommandError::io(e.to_string()))
}

/// T58: uninstall — refuses while running; consent marker is kept unless
/// withdrawn separately.
#[tauri::command(async)]
fn bridge_uninstall() -> Result<(), CommandError> {
    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .map_err(|_| CommandError::not_available("HOME directory not determinable"))?;
    let state = bridge_state_dir(&home);
    if bridge::runtime::status(&state).running {
        return Err(CommandError::invalid_input(
            "the bridge is running — stop it before uninstalling",
        ));
    }
    bridge::installer::uninstall(&bridge_install_root(&home))
        .map_err(|e| CommandError::io(e.to_string()))
}

/// T54: point Claude Code at the bridge for an OpenAI-only provider.
/// Writes settings.json via the same transactional pipeline as Save & Apply:
/// the base URL becomes the loopback gateway and the credential stays in the
/// owner-only file the bridge reads.
#[tauri::command(async)]
fn bridge_apply_mode(
    provider_id: String,
    model: Option<String>,
) -> Result<save_apply::ApplyOutcome, CommandError> {
    let _ = (provider_id, model);
    Err(CommandError::not_available(
        "The protocol bridge is unavailable; no settings were changed.",
    ))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            list_providers,
            save_and_apply,
            read_status,
            restore_last_backup,
            recover_settings,
            backup_previews,
            pending_transaction,
            switch_history,
            recover_transaction,
            check_environment,
            discover_models,
            test_connection,
            has_credential,
            get_routing,
            save_routing,
            proxy_start,
            proxy_stop,
            proxy_restart,
            proxy_status,
            helper_status,
            helper_login_start,
            config_paths,
            apply_routed_mode,
            check_routing_health,
            list_credentials,
            add_credential,
            remove_credential,
            set_credential_enabled,
            promote_credential,
            list_backups,
            bridge_consent_info,
            bridge_consent_record,
            bridge_consent_withdraw,
            bridge_overview,
            bridge_install,
            bridge_start,
            bridge_stop,
            bridge_uninstall,
            bridge_apply_mode
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
