//! Built-in provider presets (T17).
//!
//! Presets are compile-time embedded (`include_str!`) — no runtime file
//! loading for builtins, so the app cannot be tricked into loading a
//! tampered preset from disk (supply-chain discipline, threat T7).
//! A builtin that fails validation is a build-time programming error and
//! fails tests via `expect`.

use crate::provider_catalog::{ProviderTemplate, parse_template};

const ANTHROPIC_PRESET: &str = include_str!("../presets/anthropic.json");
const OPENROUTER_PRESET: &str = include_str!("../presets/openrouter.json");
const DEEPSEEK_PRESET: &str = include_str!("../presets/deepseek.json");
const ZAI_PRESET: &str = include_str!("../presets/zai.json");
const NVIDIA_NIM_PRESET: &str = include_str!("../presets/nvidia-nim.json");

/// All bundled provider templates, in display order.
pub fn load_builtin_presets() -> Vec<ProviderTemplate> {
    vec![
        parse_template(ANTHROPIC_PRESET)
            .expect("bundled anthropic preset must be valid (checked by tests)"),
        parse_template(OPENROUTER_PRESET)
            .expect("bundled openrouter preset must be valid (checked by tests)"),
        parse_template(DEEPSEEK_PRESET)
            .expect("bundled deepseek preset must be valid (checked by tests)"),
        parse_template(ZAI_PRESET).expect("bundled zai preset must be valid (checked by tests)"),
        parse_template(NVIDIA_NIM_PRESET)
            .expect("bundled nvidia-nim preset must be valid (checked by tests)"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider_catalog::{BridgeRequirement, CapabilityStatus, DiscoverySpec, Protocol};

    #[test]
    fn builtin_anthropic_preset_parses_and_validates() {
        let presets = load_builtin_presets();
        assert!(presets.len() >= 2, "anthropic + openrouter minimum");
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

    #[test]
    fn openrouter_preset_is_direct_compat_with_public_discovery() {
        let presets = load_builtin_presets();
        let t = presets
            .iter()
            .find(|p| p.id == "openrouter")
            .expect("openrouter preset present");
        assert_eq!(t.protocol, Protocol::AnthropicCompat);
        assert_eq!(t.bridge, BridgeRequirement::NotRequired);
        assert_eq!(t.auth, crate::provider_catalog::AuthStyle::BearerToken);
        assert_eq!(
            t.direct_base_url.as_deref(),
            Some("https://openrouter.ai/api")
        );
        match &t.discovery {
            DiscoverySpec::Endpoint {
                url,
                requires_auth,
                list_selector,
            } => {
                assert_eq!(url, "https://openrouter.ai/api/v1/models");
                assert!(
                    !requires_auth,
                    "models endpoint is public (live-verified HTTP 200)"
                );
                assert_eq!(list_selector, "data");
            }
            other => panic!("expected endpoint discovery, got {other:?}"),
        }
    }

    #[test]
    fn deepseek_preset_is_direct_compat_with_authenticated_discovery() {
        let presets = load_builtin_presets();
        let t = presets
            .iter()
            .find(|p| p.id == "deepseek")
            .expect("deepseek preset present");
        assert_eq!(t.protocol, Protocol::AnthropicCompat);
        assert_eq!(t.bridge, BridgeRequirement::NotRequired);
        assert_eq!(t.auth, crate::provider_catalog::AuthStyle::ApiKey);
        assert_eq!(
            t.direct_base_url.as_deref(),
            Some("https://api.deepseek.com/anthropic")
        );
        match &t.discovery {
            DiscoverySpec::Endpoint {
                url,
                requires_auth,
                list_selector,
            } => {
                assert_eq!(url, "https://api.deepseek.com/models");
                assert!(
                    requires_auth,
                    "DeepSeek /models requires credentials per docs"
                );
                assert_eq!(list_selector, "data");
            }
            other => panic!("expected endpoint discovery, got {other:?}"),
        }
    }

    #[test]
    fn zai_preset_is_direct_compat_without_discovery() {
        let presets = load_builtin_presets();
        let t = presets
            .iter()
            .find(|p| p.id == "zai")
            .expect("zai preset present");
        assert_eq!(t.protocol, Protocol::AnthropicCompat);
        assert_eq!(t.bridge, BridgeRequirement::NotRequired);
        assert_eq!(t.auth, crate::provider_catalog::AuthStyle::BearerToken);
        assert_eq!(
            t.direct_base_url.as_deref(),
            Some("https://api.z.ai/api/anthropic")
        );
        // Discovery is honestly `none`: no verified catalog endpoint.
        assert_eq!(t.discovery, DiscoverySpec::None);
    }

    #[test]
    fn nvidia_nim_preset_requires_bridge_with_public_discovery() {
        let presets = load_builtin_presets();
        let t = presets
            .iter()
            .find(|p| p.id == "nvidia-nim")
            .expect("nvidia-nim preset present");
        assert_eq!(t.protocol, Protocol::OpenAiOnly);
        assert_eq!(t.bridge, BridgeRequirement::RequiredForOpenAiOnly);
        assert_eq!(t.auth, crate::provider_catalog::AuthStyle::BearerToken);
        // No Anthropic base URL — the bridge owns the upstream URL (T24 refinement).
        assert_eq!(t.direct_base_url, None);
        match &t.discovery {
            DiscoverySpec::Endpoint {
                url,
                requires_auth,
                list_selector,
            } => {
                assert_eq!(url, "https://integrate.api.nvidia.com/v1/models");
                assert!(
                    !requires_auth,
                    "models catalog is public (live-verified HTTP 200)"
                );
                assert_eq!(list_selector, "data");
            }
            other => panic!("expected endpoint discovery, got {other:?}"),
        }
    }
}
