//! Bounded local activity: no prompts, responses, endpoints, key values or labels.
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};
static HISTORY_LOCK: Mutex<()> = Mutex::new(());
const LIMIT: usize = 100;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Selected,
    Auth,
    Quota,
    RateLimit,
    ServerError,
    Timeout,
    Network,
    Request,
    ModelUnavailable,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Event {
    pub epoch: u64,
    pub provider_id: String,
    pub outcome: Outcome,
}
fn path(home: &Path) -> std::path::PathBuf {
    home.join(".claude-config-manager/activity/history.json")
}
pub fn read(home: &Path) -> Result<Vec<Event>, String> {
    let value = match crate::config_reader::read_settings(&path(home)) {
        Ok(v) => v,
        Err(crate::config_reader::ConfigError::Missing) => return Ok(Vec::new()),
        Err(_) => {
            return Err("Activity history could not be read; existing files were preserved".into());
        }
    };
    let events: Vec<Event> = serde_json::from_value(value["events"].clone())
        .map_err(|_| "Activity history has an invalid format")?;
    if events.len() > LIMIT
        || events
            .iter()
            .any(|e| crate::credential_handoff::validate_provider_id(&e.provider_id).is_err())
    {
        return Err("Activity history exceeds its limits or has an invalid provider".into());
    }
    Ok(events)
}
pub fn append(home: &Path, provider_id: &str, outcome: Outcome) -> Result<(), String> {
    crate::credential_handoff::validate_provider_id(provider_id)
        .map_err(|_| "Invalid history provider")?;
    let _guard = HISTORY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let file = path(home);
    if crate::config_locator::ConfigLocator::new()
        .config_dir_is_symlink(&file)
        .map_err(|_| "Activity directory could not be checked")?
    {
        return Err("Activity directory contains a symlink".into());
    }
    let _lock = crate::apply_lock::ApplyLock::acquire(file.parent().unwrap())
        .map_err(|_| "Activity history is busy; event could not be recorded")?;
    let mut events = read(home)?;
    events.push(Event {
        epoch: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        provider_id: provider_id.into(),
        outcome,
    });
    if events.len() > LIMIT {
        events.drain(..events.len() - LIMIT);
    }
    let value = serde_json::json!({"version":1,"events":events});
    crate::config_writer::write_settings_atomic(&file, &value, None)
        .map_err(|_| "Activity event could not be saved".into())
}
impl From<crate::failover::FailureClass> for Outcome {
    fn from(class: crate::failover::FailureClass) -> Self {
        use crate::failover::FailureClass as F;
        match class {
            F::Auth => Self::Auth,
            F::Quota => Self::Quota,
            F::RateLimit => Self::RateLimit,
            F::ServerError => Self::ServerError,
            F::Timeout => Self::Timeout,
            F::Network => Self::Network,
            F::Request => Self::Request,
            F::Model => Self::ModelUnavailable,
        }
    }
}
