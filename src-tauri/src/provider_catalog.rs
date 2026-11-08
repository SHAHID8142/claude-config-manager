//! Provider template catalog (T16): versioned schema with direct/bridge
//! protocol metadata, parsed from embedded JSON presets.
//!
//! Contract (docs/architecture/MODULE-BOUNDARIES.md):
//! - Catalog owns template *definition* only — no network, no credentials.
//! - `schema_version` gates forward compatibility: unknown versions are
//!   rejected, never guessed at.
//! - Capability data is carried verbatim with its verification status; the
//!   catalog never fabricates values (PRD F12).

use serde::{Deserialize, Serialize};

/// Current preset schema version. Bump on any wire-shape change.
pub const PROVIDER_SCHEMA_VERSION: u32 = 1;

/// How the credential is presented to the endpoint (T02 config matrix).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthStyle {
    /// Sent as `x-api-key` header (ANTHROPIC_API_KEY).
    ApiKey,
    /// Sent as `Authorization: Bearer` header (ANTHROPIC_AUTH_TOKEN).
    BearerToken,
}

/// Wire protocol the provider speaks toward Claude Code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    /// Native Anthropic Messages API (Anthropic itself).
    AnthropicNative,
    /// Provider-hosted Anthropic-compatible endpoint (OpenRouter, DeepSeek, Z.AI).
    AnthropicCompat,
    /// OpenAI-compatible only — bridge required for Claude Code.
    OpenAiOnly,
}

/// Whether a bridge process is needed (ADR-001: direct mode first).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BridgeRequirement {
    NotRequired,
    RequiredForOpenAiOnly,
}

/// Model-catalog discovery capability, when the provider documents one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DiscoverySpec {
    /// No documented catalog endpoint; manual model ID entry only.
    None,
    /// Documented catalog endpoint; path is relative to the API root.
    Endpoint {
        /// Absolute HTTPS URL of the catalog endpoint.
        url: String,
        /// Whether the catalog call needs the credential header.
        requires_auth: bool,
        /// JSON pointer-ish selector to the array of model entries, e.g. "data".
        list_selector: String,
    },
}

/// Verification status of capability/compatibility claims (PRD F12, T25).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CapabilityStatus {
    /// Claims verified against a cited official source.
    Verified { source_url: String },
    /// Not verified — UI must display "unknown", nothing asserted.
    Unknown,
}

/// Versioned provider template. This is the catalog wire shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderTemplate {
    pub schema_version: u32,
    /// Stable identifier, e.g. "anthropic", "openrouter".
    pub id: String,
    /// Display name.
    pub name: String,
    pub auth: AuthStyle,
    pub protocol: Protocol,
    pub bridge: BridgeRequirement,
    /// Anthropic-compatible base URL for direct mode; `None` = native Anthropic
    /// (no base-url override is written; see T02 matrix).
    pub direct_base_url: Option<String>,
    pub discovery: DiscoverySpec,
    pub capability_status: CapabilityStatus,
    /// Optional official docs URL shown in the UI.
    pub docs_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CatalogError {
    pub field: &'static str,
    pub reason: String,
}

impl std::fmt::Display for CatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid provider template ({}): {}",
            self.field, self.reason
        )
    }
}

impl std::error::Error for CatalogError {}

/// Parse a template from JSON with schema gating and field validation.
pub fn parse_template(json: &str) -> Result<ProviderTemplate, CatalogError> {
    let t: ProviderTemplate = serde_json::from_str(json).map_err(|e| CatalogError {
        field: "json",
        reason: e.to_string(),
    })?;
    validate(&t)?;
    Ok(t)
}

fn field_err(field: &'static str, reason: impl Into<String>) -> CatalogError {
    CatalogError {
        field,
        reason: reason.into(),
    }
}

/// Validate semantic constraints beyond serde shapes.
pub fn validate(t: &ProviderTemplate) -> Result<(), CatalogError> {
    if t.schema_version != PROVIDER_SCHEMA_VERSION {
        return Err(field_err(
            "schema_version",
            format!(
                "unsupported schema version {} (this build understands {})",
                t.schema_version, PROVIDER_SCHEMA_VERSION
            ),
        ));
    }
    if t.id.trim().is_empty()
        || !t
            .id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c == '-' || c.is_ascii_digit())
    {
        return Err(field_err(
            "id",
            "must be non-empty lowercase kebab-case (a-z, 0-9, -)",
        ));
    }
    if t.name.trim().is_empty() {
        return Err(field_err("name", "must be non-empty"));
    }
    // URL checks: only validated when a direct base URL is claimed.
    if let Some(url) = &t.direct_base_url {
        validate_https_url(url)?;
    }
    if let DiscoverySpec::Endpoint { url, .. } = &t.discovery {
        validate_https_url(url)?;
    }
    // Protocol/bridge consistency (ADR-001).
    if t.protocol == Protocol::OpenAiOnly && t.bridge != BridgeRequirement::RequiredForOpenAiOnly {
        return Err(field_err(
            "bridge",
            "OpenAI-only protocol must declare RequiredForOpenAiOnly",
        ));
    }
    if t.bridge == BridgeRequirement::NotRequired && t.protocol == Protocol::OpenAiOnly {
        return Err(field_err(
            "bridge",
            "OpenAI-only providers cannot be direct",
        ));
    }
    if t.direct_base_url.is_none() && t.protocol != Protocol::AnthropicNative {
        return Err(field_err(
            "direct_base_url",
            "non-native protocols must declare their Anthropic-compatible base URL",
        ));
    }
    Ok(())
}

/// HTTPS-only URL validation for template-declared endpoints (threat T11).
pub fn validate_https_url(url: &str) -> Result<(), CatalogError> {
    let parsed = url::Url::parse(url).map_err(|_| field_err("url", "not a valid URL"))?;
    if parsed.scheme() != "https" {
        return Err(field_err("url", "must use https"));
    }
    if parsed.host_str().unwrap_or_default().is_empty() {
        return Err(field_err("url", "missing host"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_COMPAT: &str = r#"{
        "schema_version": 1,
        "id": "openrouter",
        "name": "OpenRouter",
        "auth": "bearer_token",
        "protocol": "anthropic_compat",
        "bridge": "not_required",
        "direct_base_url": "https://openrouter.ai/api",
        "discovery": { "kind": "endpoint", "url": "https://openrouter.ai/api/v1/models", "requires_auth": true, "list_selector": "data" },
        "capability_status": { "status": "verified", "source_url": "https://openrouter.ai/docs" },
        "docs_url": "https://openrouter.ai/docs"
    }"#;

    #[test]
    fn parses_valid_anthropic_compat_template() {
        let t = parse_template(VALID_COMPAT).expect("valid template");
        assert_eq!(t.id, "openrouter");
        assert_eq!(t.auth, AuthStyle::BearerToken);
        assert_eq!(t.protocol, Protocol::AnthropicCompat);
        assert_eq!(t.bridge, BridgeRequirement::NotRequired);
        assert_eq!(
            t.capability_status,
            CapabilityStatus::Verified {
                source_url: "https://openrouter.ai/docs".into()
            }
        );
    }

    #[test]
    fn rejects_unknown_schema_version() {
        let json = VALID_COMPAT.replace("\"schema_version\": 1", "\"schema_version\": 99");
        let err = parse_template(&json).expect_err("must reject future schema");
        assert_eq!(err.field, "schema_version");
    }

    #[test]
    fn rejects_missing_field() {
        let json = VALID_COMPAT.replace("\"name\": \"OpenRouter\",", "");
        let err = parse_template(&json).expect_err("must reject missing field");
        assert_eq!(err.field, "json");
    }

    #[test]
    fn rejects_non_https_base_url() {
        let json = VALID_COMPAT.replace("https://openrouter.ai/api", "http://openrouter.ai/api");
        let err = parse_template(&json).expect_err("must reject http://");
        assert_eq!(err.field, "url");
    }

    #[test]
    fn rejects_bad_id_characters() {
        let json = VALID_COMPAT.replace("\"id\": \"openrouter\"", "\"id\": \"Open Router!\"");
        let err = parse_template(&json).expect_err("must reject bad id");
        assert_eq!(err.field, "id");
    }

    #[test]
    fn rejects_openai_only_without_bridge_metadata() {
        let json = r#"{
            "schema_version": 1,
            "id": "nim",
            "name": "NVIDIA NIM",
            "auth": "bearer_token",
            "protocol": "open_ai_only",
            "bridge": "not_required",
            "direct_base_url": "https://integrate.api.nvidia.com",
            "discovery": { "kind": "none" },
            "capability_status": { "status": "unknown" }
        }"#;
        let err = parse_template(json).expect_err("openai-only must require bridge");
        assert_eq!(err.field, "bridge");
    }

    #[test]
    fn rejects_compat_without_base_url() {
        let json = VALID_COMPAT.replace("\"direct_base_url\": \"https://openrouter.ai/api\",", "");
        let err = parse_template(&json).expect_err("compat needs base url");
        assert_eq!(err.field, "direct_base_url");
    }

    #[test]
    fn rejects_discovery_url_not_https() {
        let json = VALID_COMPAT.replace(
            "https://openrouter.ai/api/v1/models",
            "ftp://openrouter.ai/api/v1/models",
        );
        let err = parse_template(&json).expect_err("discovery must be https");
        assert_eq!(err.field, "url");
    }

    #[test]
    fn https_validator_rejects_garbage() {
        assert!(validate_https_url("not a url").is_err());
        assert!(validate_https_url("https://").is_err());
        assert!(validate_https_url("https://example.com/path").is_ok());
    }
}
