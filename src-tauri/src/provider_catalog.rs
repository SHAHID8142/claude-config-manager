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
    /// Optional credential style for the **model-catalog** call when it differs
    /// from the chat credential style (T44b). Absent = use `auth`.
    ///
    /// Verified example: DeepSeek's Anthropic-compatible chat endpoint takes
    /// the key as `x-api-key`, but its OpenAI-style `GET /models` catalog
    /// requires `Authorization: Bearer` (api-docs.deepseek.com/api/list-models).
    /// One field cannot express both, and guessing the wrong one silently
    /// breaks discovery for every user.
    #[serde(default)]
    pub discovery_auth: Option<AuthStyle>,
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
    // Base-URL policy per protocol (T24 refinement): native Anthropic must
    // not override; anthropic_compat must declare its compat endpoint;
    // open_ai_only has no Anthropic base URL at all — the bridge owns the
    // upstream URL.
    match t.protocol {
        Protocol::AnthropicNative => {
            if t.direct_base_url.is_some() {
                return Err(field_err(
                    "direct_base_url",
                    "native Anthropic must not override the base URL",
                ));
            }
        }
        Protocol::AnthropicCompat => {
            if t.direct_base_url.is_none() {
                return Err(field_err(
                    "direct_base_url",
                    "anthropic_compat protocols must declare their Anthropic-compatible base URL",
                ));
            }
        }
        Protocol::OpenAiOnly => {
            if t.direct_base_url.is_some() {
                return Err(field_err(
                    "direct_base_url",
                    "open_ai_only providers must not declare an Anthropic base URL (bridge owns the upstream URL)",
                ));
            }
        }
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

/// UI-facing summary of a provider template (T18 command contract).
/// camelCase on the wire to match TS conventions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSummary {
    pub id: String,
    pub name: String,
    pub protocol: Protocol,
    pub bridge: BridgeRequirement,
    pub capability_status: CapabilityStatus,
    pub docs_url: Option<String>,
    /// Direct base URL for compat presets; None for native/OpenAI-only (T33).
    pub direct_base_url: Option<String>,
}

impl ProviderSummary {
    pub fn from_template(t: &ProviderTemplate) -> Self {
        Self {
            id: t.id.clone(),
            name: t.name.clone(),
            protocol: t.protocol,
            bridge: t.bridge,
            capability_status: t.capability_status.clone(),
            docs_url: t.docs_url.clone(),
            direct_base_url: t.direct_base_url.clone(),
        }
    }
}

/// Classification of a user-provided endpoint URL (T19).
/// Enforced in Rust (apply-time authority) and mirrored in TS for live UI
/// feedback; both sides must stay in sync (contract tests in T19 audits).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "check", content = "detail", rename_all = "snake_case")]
pub enum EndpointCheck {
    /// Not parseable as a URL.
    Invalid(String),
    /// Parses but does not use https — credentials would travel in cleartext.
    Insecure,
    /// https pointing at this machine — fine for local gateways; noted so the
    /// user knows traffic stays local.
    Local,
    /// https remote endpoint — acceptable.
    RemoteOk,
}

/// Classify an endpoint for security feedback (threat T11).
pub fn classify_endpoint(url: &str) -> EndpointCheck {
    // T59: the URL parser silently strips tabs/newlines, so
    // "example.com\n.evil.com" would become a different host than the one the
    // user sees. Refuse such input before it can be normalised into something
    // else.
    if url.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return EndpointCheck::Invalid(
            "URL must not contain whitespace or control characters".into(),
        );
    }
    let parsed = match url::Url::parse(url) {
        Ok(p) => p,
        Err(e) => return EndpointCheck::Invalid(e.to_string()),
    };
    match parsed.scheme() {
        "https" => {}
        "http" => {
            // Plaintext transport for a credential-bearing endpoint.
            return EndpointCheck::Insecure;
        }
        // T59: `javascript:`, `data:`, `file:`, `ftp:` and friends are not
        // "insecure http" — they are not endpoints at all. Reporting them as
        // Insecure would tell the user "use https" for a value that can never
        // work, so they are classified Invalid with the scheme named.
        other => {
            return EndpointCheck::Invalid(format!(
                "unsupported URL scheme {other:?} — only https is accepted"
            ));
        }
    }
    // T59: a URL carrying `user:pass@` would write a live credential into
    // settings.json (plaintext, and backed up). The app owns credentials via
    // Keychain, so an endpoint that embeds one is refused outright.
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return EndpointCheck::Invalid(
            "URL must not embed credentials (user:pass@) — the API key is stored separately".into(),
        );
    }
    let host = parsed.host_str().unwrap_or_default().to_ascii_lowercase();
    let is_local = matches!(host.as_str(), "localhost" | "127.0.0.1" | "::1" | "[::1]")
        || host.ends_with(".local");
    if is_local {
        EndpointCheck::Local
    } else {
        EndpointCheck::RemoteOk
    }
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
            "direct_base_url": null,
            "discovery": { "kind": "none" },
            "capability_status": { "status": "unknown" }
        }"#;
        let err = parse_template(json).expect_err("openai-only must require bridge");
        assert_eq!(err.field, "bridge");
    }

    #[test]
    fn rejects_openai_only_with_anthropic_base_url() {
        let json = r#"{
            "schema_version": 1,
            "id": "nim",
            "name": "NVIDIA NIM",
            "auth": "bearer_token",
            "protocol": "open_ai_only",
            "bridge": "required_for_open_ai_only",
            "direct_base_url": "https://integrate.api.nvidia.com",
            "discovery": { "kind": "none" },
            "capability_status": { "status": "unknown" }
        }"#;
        let err = parse_template(json).expect_err("openai-only must not set Anthropic base URL");
        assert_eq!(err.field, "direct_base_url");
    }

    #[test]
    fn rejects_native_with_base_url_override() {
        let json = VALID_COMPAT
            .replace("anthropic_compat", "anthropic_native")
            .replace("required_for_open_ai_only", "not_required")
            .replace(
                "\"direct_base_url\": \"https://openrouter.ai/api\",",
                "\"direct_base_url\": \"https://evil.example.com\",",
            );
        let err = parse_template(&json).expect_err("native must not override base URL");
        assert_eq!(err.field, "direct_base_url");
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

    #[test]
    fn endpoint_check_classifies_https_remote_as_ok() {
        assert_eq!(
            classify_endpoint("https://api.example.com/v1"),
            EndpointCheck::RemoteOk
        );
    }

    #[test]
    fn endpoint_check_classifies_http_as_insecure() {
        assert_eq!(
            classify_endpoint("http://api.example.com"),
            EndpointCheck::Insecure
        );
        // T59: non-http schemes are not "insecure transport", they are not
        // endpoints at all — reporting Insecure would be misleading advice.
        assert!(matches!(
            classify_endpoint("ftp://x"),
            EndpointCheck::Invalid(_)
        ));
    }

    #[test]
    fn endpoint_check_classifies_localhost_as_local() {
        assert_eq!(
            classify_endpoint("https://localhost:8080"),
            EndpointCheck::Local
        );
        assert_eq!(
            classify_endpoint("https://127.0.0.1:31415"),
            EndpointCheck::Local
        );
        assert_eq!(
            classify_endpoint("https://mybox.local"),
            EndpointCheck::Local
        );
    }

    #[test]
    fn endpoint_check_classifies_garbage_as_invalid() {
        assert!(matches!(
            classify_endpoint("::: not a url"),
            EndpointCheck::Invalid(_)
        ));
        assert!(matches!(classify_endpoint(""), EndpointCheck::Invalid(_)));
    }

    #[test]
    fn discovery_auth_override_is_optional_and_parsed() {
        // Absent → None: the chat credential style is used for the catalog.
        let t = parse_template(VALID_COMPAT).expect("valid");
        assert_eq!(t.discovery_auth, None);

        // Present → parsed verbatim (T44b: DeepSeek chat=x-api-key, catalog=Bearer).
        let with = VALID_COMPAT.replace(
            "\"auth\": \"bearer_token\",",
            "\"auth\": \"api_key\", \"discovery_auth\": \"bearer_token\",",
        );
        let t = parse_template(&with).expect("valid with override");
        assert_eq!(t.auth, AuthStyle::ApiKey);
        assert_eq!(t.discovery_auth, Some(AuthStyle::BearerToken));
    }

    #[test]
    fn discovery_auth_override_rejects_unknown_style() {
        let bad = VALID_COMPAT.replace(
            "\"auth\": \"bearer_token\",",
            "\"auth\": \"bearer_token\", \"discovery_auth\": \"magic\",",
        );
        let err = parse_template(&bad).expect_err("unknown style must be rejected");
        assert_eq!(err.field, "json");
    }

    #[test]
    fn endpoint_check_serializes_tagged_shape() {
        let json = serde_json::to_value(classify_endpoint("https://x.com")).expect("ser");
        assert_eq!(json["check"], "remote_ok");
        let json = serde_json::to_value(classify_endpoint("http://x.com")).expect("ser");
        assert_eq!(json["check"], "insecure");
    }
}
