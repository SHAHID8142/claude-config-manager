//! V2-23/28: routing targets — the ordered chain the loopback proxy tries.
//!
//! Stored at `~/.claude-config-manager/routing.json` (0600). It contains **no
//! key material**: only which credential id and model to use per target. The
//! proxy resolves the actual secret through [`crate::credential_vault`].
//!
//! Validation is strict and happens before any write, because a half-valid
//! chain would be indistinguishable from a working one at request time.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

pub const ROUTING_VERSION: u32 = 1;
const MAX_TARGETS: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RouteTarget {
    /// Stable id, `[a-z0-9-]{1,64}` (also used in status/diagnostics).
    pub id: String,
    pub provider_id: String,
    /// Which stored credential to use. `None` = the provider's legacy file.
    pub credential_id: Option<String>,
    /// Model id to force for this target. `None` = pass the caller's model.
    pub model: Option<String>,
    /// Upstream base URL, without `/v1/messages`.
    pub base_url: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RoutingConfig {
    pub version: u32,
    /// Failover is opt-in; `false` means Claude Code talks to the provider直接
    /// (the app never invents a proxy hop).
    pub enabled: bool,
    pub targets: Vec<RouteTarget>,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            version: ROUTING_VERSION,
            enabled: false,
            targets: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub enum RoutingError {
    Format(String),
    Invalid(String),
    Io(String),
}

impl std::fmt::Display for RoutingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoutingError::Format(m) | RoutingError::Invalid(m) | RoutingError::Io(m) => {
                f.write_str(m)
            }
        }
    }
}

impl RoutingConfig {
    pub fn parse(raw: &str) -> Result<Self, RoutingError> {
        let cfg: RoutingConfig = serde_json::from_str(raw)
            .map_err(|e| RoutingError::Format(format!("routing.json: {e}")))?;
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn to_json(&self) -> Result<String, RoutingError> {
        self.validate()?;
        let mut s =
            serde_json::to_string_pretty(self).map_err(|e| RoutingError::Format(e.to_string()))?;
        s.push('\n');
        Ok(s)
    }

    pub fn validate(&self) -> Result<(), RoutingError> {
        if self.version != ROUTING_VERSION {
            return Err(RoutingError::Format(format!(
                "unsupported routing.json version {} (this build writes {ROUTING_VERSION})",
                self.version
            )));
        }
        if self.targets.len() > MAX_TARGETS {
            return Err(RoutingError::Invalid(format!(
                "at most {MAX_TARGETS} routing targets"
            )));
        }
        let mut seen: Vec<&str> = Vec::new();
        for t in &self.targets {
            crate::credential_index::validate_credential_id(&t.id)
                .map_err(|_| RoutingError::Invalid(format!("invalid target id {}", t.id)))?;
            if seen.contains(&t.id.as_str()) {
                return Err(RoutingError::Invalid(format!(
                    "duplicate target id {}",
                    t.id
                )));
            }
            seen.push(&t.id);
            crate::credential_handoff::validate_provider_id(&t.provider_id).map_err(|_| {
                RoutingError::Invalid(format!("target {}: invalid provider id", t.id))
            })?;
            if let Some(cred) = &t.credential_id {
                crate::credential_index::validate_credential_id(cred).map_err(|_| {
                    RoutingError::Invalid(format!("target {}: invalid credential id", t.id))
                })?;
            }
            if let Some(model) = &t.model {
                crate::config_merge::validate_model_id(model)
                    .map_err(|e| RoutingError::Invalid(format!("target {}: {e}", t.id)))?;
            }
            // Upstream must be https, except a loopback http endpoint, which
            // exists only so tests can run a local upstream. A public http
            // endpoint would send the key in clear text.
            match crate::provider_catalog::classify_endpoint(&t.base_url) {
                crate::provider_catalog::EndpointCheck::RemoteOk => {}
                // Loopback upstreams are legitimate (tests, local gateways).
                crate::provider_catalog::EndpointCheck::Local => {}
                crate::provider_catalog::EndpointCheck::Insecure => {
                    let loopback = t.base_url.starts_with("http://127.0.0.1")
                        || t.base_url.starts_with("http://localhost");
                    if !loopback {
                        return Err(RoutingError::Invalid(format!(
                            "target {}: base URL must be https (http is only allowed for loopback)",
                            t.id
                        )));
                    }
                }
                crate::provider_catalog::EndpointCheck::Invalid(m) => {
                    return Err(RoutingError::Invalid(format!("target {}: {m}", t.id)));
                }
            }
        }
        Ok(())
    }

    /// Enabled targets in configured order.
    pub fn ordered(&self) -> Vec<&RouteTarget> {
        self.targets.iter().filter(|t| t.enabled).collect()
    }

    /// Move a target up one slot (V2-24 priority editing).
    pub fn move_up(&mut self, id: &str) -> bool {
        let Some(i) = self.targets.iter().position(|t| t.id == id) else {
            return false;
        };
        if i == 0 {
            return false;
        }
        self.targets.swap(i - 1, i);
        true
    }

    pub fn move_down(&mut self, id: &str) -> bool {
        let Some(i) = self.targets.iter().position(|t| t.id == id) else {
            return false;
        };
        if i + 1 >= self.targets.len() {
            return false;
        }
        self.targets.swap(i, i + 1);
        true
    }
}

/// `<home>/.claude-config-manager/routing.json`
pub fn routing_path(home: &Path) -> PathBuf {
    home.join(".claude-config-manager").join("routing.json")
}

pub fn load(home: &Path) -> Result<RoutingConfig, RoutingError> {
    match fs::read_to_string(routing_path(home)) {
        Ok(raw) => RoutingConfig::parse(&raw),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(RoutingConfig::default()),
        Err(e) => Err(RoutingError::Io(e.to_string())),
    }
}

pub fn save(home: &Path, cfg: &RoutingConfig) -> Result<(), RoutingError> {
    let json = cfg.to_json()?;
    let path = routing_path(home);
    let dir = path
        .parent()
        .ok_or_else(|| RoutingError::Io("no parent dir".into()))?;
    fs::create_dir_all(dir).map_err(|e| RoutingError::Io(e.to_string()))?;
    let tmp = path.with_extension("json.tmp");
    {
        let mut f = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(&tmp)
            .map_err(|e| RoutingError::Io(e.to_string()))?;
        f.write_all(json.as_bytes())
            .map_err(|e| RoutingError::Io(e.to_string()))?;
        f.sync_all().map_err(|e| RoutingError::Io(e.to_string()))?;
    }
    fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600))
        .map_err(|e| RoutingError::Io(e.to_string()))?;
    fs::rename(&tmp, &path).map_err(|e| RoutingError::Io(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static N: AtomicU32 = AtomicU32::new(0);

    fn target(id: &str) -> RouteTarget {
        RouteTarget {
            id: id.into(),
            provider_id: "openrouter".into(),
            credential_id: Some("openrouter-1".into()),
            model: Some("nvidia/nemotron-x".into()),
            base_url: "https://openrouter.ai/api".into(),
            enabled: true,
        }
    }

    fn cfg(targets: Vec<RouteTarget>) -> RoutingConfig {
        RoutingConfig {
            version: ROUTING_VERSION,
            enabled: true,
            targets,
        }
    }

    fn temp_home(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ccm-routing-{}-{}-{name}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("home");
        dir
    }

    #[test]
    fn round_trips_and_contains_no_key_material() {
        let c = cfg(vec![target("t1")]);
        let json = c.to_json().expect("json");
        assert!(!json.contains("sk-"));
        assert_eq!(RoutingConfig::parse(&json).expect("parse"), c);
    }

    #[test]
    fn failsafe_default_is_disabled_and_empty() {
        let d = RoutingConfig::default();
        assert!(!d.enabled);
        assert!(d.targets.is_empty());
        assert!(d.ordered().is_empty());
    }

    #[test]
    fn refuses_plain_http_remotes_but_allows_loopback_for_tests() {
        let mut bad = cfg(vec![target("t1")]);
        bad.targets[0].base_url = "http://openrouter.ai/api".into();
        assert!(matches!(bad.validate(), Err(RoutingError::Invalid(_))));

        let mut ok = cfg(vec![target("t1")]);
        ok.targets[0].base_url = "http://127.0.0.1:41234".into();
        assert!(ok.validate().is_ok());
    }

    #[test]
    fn refuses_duplicate_ids_bad_models_and_bad_versions() {
        let dup = cfg(vec![target("t1"), target("t1")]);
        assert!(matches!(dup.validate(), Err(RoutingError::Invalid(_))));

        let mut bad_model = cfg(vec![target("t1")]);
        bad_model.targets[0].model = Some("has space".into());
        assert!(matches!(
            bad_model.validate(),
            Err(RoutingError::Invalid(_))
        ));

        let mut bad_version = cfg(vec![target("t1")]);
        bad_version.version = 2;
        assert!(matches!(
            bad_version.validate(),
            Err(RoutingError::Format(_))
        ));
    }

    #[test]
    fn reordering_moves_one_slot_and_stops_at_the_edges() {
        let mut c = cfg(vec![target("t1"), target("t2"), target("t3")]);
        assert!(c.move_up("t2"));
        assert_eq!(
            c.ordered()
                .iter()
                .map(|t| t.id.as_str())
                .collect::<Vec<_>>(),
            ["t2", "t1", "t3"]
        );
        assert!(!c.move_up("t2"), "already first");
        assert!(c.move_down("t1"));
        assert_eq!(
            c.ordered()
                .iter()
                .map(|t| t.id.as_str())
                .collect::<Vec<_>>(),
            ["t2", "t3", "t1"]
        );
        assert!(!c.move_down("t1"), "already last");
        assert!(!c.move_up("missing"));
    }

    #[test]
    fn save_is_0600_and_load_returns_default_when_missing() {
        let home = temp_home("save");
        assert!(!load(&home).expect("default").enabled);
        let c = cfg(vec![target("t1")]);
        save(&home, &c).expect("save");
        let mode = fs::metadata(routing_path(&home))
            .expect("meta")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
        assert_eq!(load(&home).expect("load"), c);
    }

    #[test]
    fn disabled_targets_are_never_returned_as_candidates() {
        let mut c = cfg(vec![target("t1"), target("t2")]);
        c.targets[1].enabled = false;
        assert_eq!(c.ordered().len(), 1);
        assert_eq!(c.ordered()[0].id, "t1");
    }
}
