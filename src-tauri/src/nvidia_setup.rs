//! Activate NVIDIA through the built-in detached translation helper.
use crate::{
    config_locator::ConfigLocator,
    config_merge::{DesiredSettings, apply_desired},
    config_reader::{self, ConfigError},
    credential_vault::CredentialVault,
    helper,
    routing_config::{self, RouteTarget},
    save_apply::{ApplyError, ApplyOutcome, RoleOverrides, SaveApplyService},
};
const ENDPOINT: &str = "https://integrate.api.nvidia.com";
/// One shared lock and private journal cover keys, policy, helper and settings.
/// The callback permits isolated tests without launching a GUI or contacting a provider.
#[allow(clippy::too_many_arguments)]
pub fn apply(
    svc: &SaveApplyService,
    model: &str,
    key: Option<&str>,
    roles: &RoleOverrides,
    context: Option<u32>,
    reset: bool,
    ready: impl FnOnce() -> Result<u16, String>,
) -> Result<ApplyOutcome, ApplyError> {
    let model = model.trim();
    crate::config_merge::validate_model_id(model)
        .map_err(|e| ApplyError::InvalidInput(e.to_string()))?;
    if model.is_empty() {
        return Err(ApplyError::InvalidInput(
            "Choose a NVIDIA model before Save & Apply".into(),
        ));
    }
    let path = ConfigLocator::new().locate(&svc.home, svc.config_dir_override.as_deref());
    let current = match config_reader::read_settings(&path) {
        Ok(v) => v,
        Err(ConfigError::Missing) => serde_json::json!({}),
        Err(e) => return Err(ApplyError::Config(e)),
    };
    let preview = DesiredSettings {
        model: Some(model.into()),
        env_default_haiku: roles.haiku.clone(),
        env_default_sonnet: roles.sonnet.clone(),
        env_default_opus: roles.opus.clone(),
        clear_env_roles: roles.reset,
        env_max_context_tokens: context.map(|n| n.to_string()),
        clear_env_max_context_tokens: reset,
        ..Default::default()
    };
    apply_desired(&current, &preview).map_err(ApplyError::Config)?;
    let mut transaction =
        crate::transaction_journal::ManagedTransaction::begin(&svc.home, Some(&path))
            .map_err(ApplyError::Io)?;
    let result = (|| {
        let before = routing_config::load(&svc.home).map_err(|e| ApplyError::Io(e.to_string()))?;
        let mut staged = before.clone();
        staged.enabled = true;
        staged.targets.retain(|t| t.provider_id != "nvidia-nim");
        staged.targets.insert(
            0,
            RouteTarget {
                id: "nvidia-default".into(),
                provider_id: "nvidia-nim".into(),
                credential_id: None,
                model: Some(model.into()),
                base_url: ENDPOINT.into(),
                enabled: true,
            },
        );
        staged
            .validate()
            .map_err(|e| ApplyError::InvalidInput(e.to_string()))?;
        let vault = CredentialVault::for_home(&svc.home);
        if let Some(key) = key.filter(|s| !s.trim().is_empty()) {
            let record = vault
                .add_locked("nvidia-nim", "Added from setup", key, &mut transaction)
                .map_err(|e| ApplyError::Io(e.to_string()))?;
            vault
                .promote_locked(&record.id, &mut transaction)
                .map_err(|e| ApplyError::Io(e.to_string()))?;
        }
        if !vault
            .has_any("nvidia-nim")
            .map_err(|e| ApplyError::Io(e.to_string()))?
        {
            return Err(ApplyError::NoCredential);
        }
        helper::ensure_token_locked(&svc.home, &mut transaction).map_err(ApplyError::Io)?;
        routing_config::save_locked(&svc.home, &staged, transaction.lock())
            .map_err(|e| ApplyError::Io(e.to_string()))?;
        let port = ready().map_err(ApplyError::Io)?;
        svc.apply_with_window_locked(
            "proxy",
            Some(model),
            Some(&format!("http://127.0.0.1:{port}")),
            None,
            roles,
            context,
            reset,
            Some(helper::TOKEN_ACCOUNT),
            &mut transaction,
        )
    })();
    SaveApplyService::finish_transaction(transaction, result)
}
