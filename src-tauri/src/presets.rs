//! Built-in provider presets (T17).
//!
//! Presets are compile-time embedded (`include_str!`) — no runtime file
//! loading for builtins, so the app cannot be tricked into loading a
//! tampered preset from disk (supply-chain discipline, threat T7).
//! A builtin that fails validation is a build-time programming error and
//! fails tests via `expect`.

use crate::provider_catalog::{ProviderTemplate, parse_template};

const ANTHROPIC_PRESET: &str = include_str!("../presets/anthropic.json");

/// All bundled provider templates, in display order.
pub fn load_builtin_presets() -> Vec<ProviderTemplate> {
    vec![
        parse_template(ANTHROPIC_PRESET)
            .expect("bundled anthropic preset must be valid (checked by tests)"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider_catalog::{BridgeRequirement, CapabilityStatus, DiscoverySpec, Protocol};

    #[test]
    fn builtin_anthropic_preset_parses_and_validates() {
        let presets = load_builtin_presets();
        assert_eq!(presets.len(), 1);
        let t = &presets[0];
        assert_eq!(t.id, "anthropic");
        assert_eq!(t.name, "Anthropic");
        assert_eq!(t.protocol, Protocol::AnthropicNative);
        assert_eq!(t.bridge, BridgeRequirement::NotRequired);
        assert_eq!(
            t.direct_base_url, None,
            "native Anthropic must not override base URL"
        );
        assert!(t.auth == crate::provider_catalog::AuthStyle::ApiKey);
    }

    #[test]
    fn anthropic_discovery_is_documented_models_endpoint() {
        let t = &load_builtin_presets()[0];
        match &t.discovery {
            DiscoverySpec::Endpoint {
                url,
                requires_auth,
                list_selector,
            } => {
                assert_eq!(url, "https://api.anthropic.com/v1/models");
                assert!(requires_auth);
                assert_eq!(list_selector, "data");
            }
            other => panic!("expected endpoint discovery, got {other:?}"),
        }
    }

    #[test]
    fn anthropic_capability_is_verified_with_source() {
        let t = &load_builtin_presets()[0];
        assert_eq!(
            t.capability_status,
            CapabilityStatus::Verified {
                source_url: "https://platform.claude.com/docs/en/api/models-list".into()
            }
        );
    }

    #[test]
    fn builtin_presets_have_unique_ids() {
        let presets = load_builtin_presets();
        let mut ids: Vec<_> = presets.iter().map(|p| p.id.clone()).collect();
        ids.sort();
        let len = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), len, "preset ids must be unique");
    }
}
