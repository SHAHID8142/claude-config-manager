//! Model-catalog discovery interface (T37, PRD F04/F05).
//!
//! Contract:
//! - Fetches ONLY documented catalog endpoints taken from verified provider
//!   templates (`DiscoverySpec::Endpoint`); the app never invents URLs.
//! - HTTPS enforced for every non-loopback endpoint (threat T11); loopback
//!   http is permitted for the local mock-server test fixtures only.
//! - Responses are schema-validated: `list_selector` must resolve to an
//!   array of objects with string `id`s; payload size and entry count are
//!   capped; entries carry NO capability claims (unknown by default, F12).
//! - NEVER auto-selects anything — discovery only lists (PRD F04).

use crate::provider_catalog::{AuthStyle, DiscoverySpec};
use serde::Serialize;
use std::io::Read;
use std::time::Duration;

/// Response size cap (providers publish catalogs up to a few MB).
pub const MAX_RESPONSE_BYTES: usize = 5 * 1024 * 1024;
/// Entry-count cap (defensive against hostile catalogs).
pub const MAX_MODELS: usize = 1000;
/// Request timeout.
pub const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelEntry {
    /// Provider-native model identifier (the only thing we ever write).
    pub id: String,
    /// Optional display name from the catalog (display only, never written).
    pub display_name: Option<String>,
    /// Always `unknown` at discovery time — capabilities are never invented.
    pub capability_status: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DiscoveryError {
    /// Endpoint failed validation (non-https, malformed).
    InvalidEndpoint(String),
    /// Transport failure (DNS, connect, timeout) — message pre-redacted.
    Transport(String),
    /// Non-success status from the endpoint.
    HttpStatus(u16),
    /// Body exceeded the size cap or was not JSON / wrong shape.
    InvalidResponse(String),
    /// Credential required by the spec but not supplied.
    CredentialRequired,
}

impl std::fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscoveryError::InvalidEndpoint(m) => write!(f, "invalid catalog endpoint ({m})"),
            DiscoveryError::Transport(m) => write!(f, "catalog request failed ({m})"),
            DiscoveryError::HttpStatus(code) => write!(f, "catalog endpoint returned HTTP {code}"),
            DiscoveryError::InvalidResponse(m) => write!(f, "catalog response unusable ({m})"),
            DiscoveryError::CredentialRequired => {
                write!(
                    f,
                    "this catalog requires a credential — save the API key first"
                )
            }
        }
    }
}

/// Is this host loopback (mock fixtures / local gateways)?
fn is_loopback_host(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "::1" | "[::1]")
}

/// Validate the endpoint URL from a DiscoverySpec (https or loopback-http).
pub fn validate_endpoint(url: &str) -> Result<(), DiscoveryError> {
    let parsed =
        url::Url::parse(url).map_err(|e| DiscoveryError::InvalidEndpoint(e.to_string()))?;
    let host = parsed.host_str().unwrap_or_default();
    if parsed.scheme() == "https" && !host.is_empty() {
        return Ok(());
    }
    if parsed.scheme() == "http" && is_loopback_host(host) {
        return Ok(());
    }
    Err(DiscoveryError::InvalidEndpoint(format!(
        "must be https (loopback http allowed only for local fixtures), got {host}"
    )))
}

/// Fetch and parse a model catalog per the template's discovery spec.
pub fn fetch_models(
    spec: &DiscoverySpec,
    auth_style: AuthStyle,
    credential: Option<&str>,
) -> Result<Vec<ModelEntry>, DiscoveryError> {
    let DiscoverySpec::Endpoint {
        url,
        requires_auth,
        list_selector,
    } = spec
    else {
        return Err(DiscoveryError::InvalidEndpoint(
            "provider template declares no catalog endpoint".into(),
        ));
    };
    validate_endpoint(url)?;

    if *requires_auth && credential.map(|c| c.trim().is_empty()).unwrap_or(true) {
        return Err(DiscoveryError::CredentialRequired);
    }

    let body = http_get(url, auth_style, credential)?;

    // Size guard enforced before parse (body already capped during read).
    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| DiscoveryError::InvalidResponse(format!("not JSON: {}", e)))?;

    // Resolve the list via the documented selector (single-level key or
    // dotted path, e.g. "data").
    let mut node = &json;
    for part in list_selector.split('.') {
        node = node
            .get(part)
            .ok_or_else(|| DiscoveryError::InvalidResponse(format!("missing key '{part}'")))?;
    }
    let list = node.as_array().ok_or_else(|| {
        DiscoveryError::InvalidResponse(format!("'{list_selector}' is not an array"))
    })?;

    if list.len() > MAX_MODELS {
        return Err(DiscoveryError::InvalidResponse(format!(
            "catalog exceeds {MAX_MODELS} entries"
        )));
    }

    let mut out = Vec::with_capacity(list.len().min(MAX_MODELS));
    for entry in list {
        let Some(obj) = entry.as_object() else {
            continue; // skip non-object entries rather than failing the whole list
        };
        let Some(id) = obj.get("id").and_then(|v| v.as_str()) else {
            continue; // entries without an id are unusable
        };
        if id.is_empty() || id.len() > 256 || id.chars().any(|c| c.is_control()) {
            continue;
        }
        out.push(ModelEntry {
            id: id.to_string(),
            display_name: obj
                .get("display_name")
                .or_else(|| obj.get("name"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            capability_status: "unknown",
        });
    }
    Ok(out)
}

fn http_get(
    url: &str,
    auth_style: AuthStyle,
    credential: Option<&str>,
) -> Result<String, DiscoveryError> {
    // Agent with a global timeout (ureq v3 config API).
    let config = ureq::config::Config::builder()
        .timeout_global(Some(DISCOVERY_TIMEOUT))
        .build();
    let agent = ureq::Agent::new_with_config(config);

    let req = agent.get(url).header("accept", "application/json");
    let req = match credential {
        Some(cred) => match auth_style {
            AuthStyle::ApiKey => req.header("x-api-key", cred),
            AuthStyle::BearerToken => req.header("authorization", &format!("Bearer {cred}")),
        },
        None => req,
    };

    let response = req.call().map_err(|e| match e {
        ureq::Error::StatusCode(code) => DiscoveryError::HttpStatus(code),
        other => DiscoveryError::Transport(redact_transport(&other.to_string())),
    })?;

    // Read with the hard size cap.
    let mut capped = Vec::with_capacity(64 * 1024);
    let mut reader = response
        .into_body()
        .into_reader()
        .take(MAX_RESPONSE_BYTES as u64);
    reader
        .read_to_end(&mut capped)
        .map_err(|e| DiscoveryError::Transport(format!("body read failed: {e}")))?;
    if capped.len() >= MAX_RESPONSE_BYTES {
        return Err(DiscoveryError::InvalidResponse("response too large".into()));
    }
    String::from_utf8(capped).map_err(|_| DiscoveryError::InvalidResponse("body not UTF-8".into()))
}

/// Transport error strings can embed URLs — strip query strings defensively.
fn redact_transport(msg: &str) -> String {
    msg.split('?').next().unwrap_or(msg).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    /// Minimal one-shot loopback HTTP fixture (no dependencies).
    fn spawn_one_shot(response: String) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let addr = listener.local_addr().expect("addr");
        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf);
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
        });
        format!("http://{addr}/models")
    }

    const OPENAI_SHAPE: &str = r#"{"object":"list","data":[{"id":"model-a","name":"Model A"},{"id":"model-b","display_name":"B"},{"id":"","name":"dropped"},{"name":"no-id-dropped"},123]}"#;

    #[test]
    fn endpoint_validation_enforces_https_or_loopback() {
        assert!(validate_endpoint("https://api.example.com/v1/models").is_ok());
        assert!(validate_endpoint("http://127.0.0.1:9/v1/models").is_ok());
        assert!(validate_endpoint("http://localhost:9/x").is_ok());
        assert!(validate_endpoint("http://api.example.com/v1/models").is_err());
        assert!(validate_endpoint("ftp://x").is_err());
        assert!(validate_endpoint("not a url").is_err());
    }

    #[test]
    fn missing_credential_for_auth_catalog_refused() {
        let spec = DiscoverySpec::Endpoint {
            url: "https://api.example.com/v1/models".into(),
            requires_auth: true,
            list_selector: "data".into(),
        };
        assert_eq!(
            fetch_models(&spec, AuthStyle::ApiKey, None),
            Err(DiscoveryError::CredentialRequired)
        );
        assert_eq!(
            fetch_models(&spec, AuthStyle::ApiKey, Some("   ")),
            Err(DiscoveryError::CredentialRequired)
        );
    }

    #[test]
    fn no_endpoint_spec_refused() {
        let spec = DiscoverySpec::None;
        assert!(matches!(
            fetch_models(&spec, AuthStyle::ApiKey, None),
            Err(DiscoveryError::InvalidEndpoint(_))
        ));
    }

    #[test]
    fn parses_mock_openai_shape_with_entry_filtering() {
        let url = spawn_one_shot(format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{OPENAI_SHAPE}"
        ));
        let spec = DiscoverySpec::Endpoint {
            url,
            requires_auth: false,
            list_selector: "data".into(),
        };
        let models = fetch_models(&spec, AuthStyle::ApiKey, None).expect("fetch");
        assert_eq!(models.len(), 2, "empty-id and non-object entries dropped");
        assert_eq!(models[0].id, "model-a");
        assert_eq!(models[0].display_name.as_deref(), Some("Model A"));
        assert_eq!(models[1].id, "model-b");
        assert_eq!(models[1].display_name.as_deref(), Some("B"));
        // Capability claims are never invented.
        assert_eq!(models[0].capability_status, "unknown");
    }

    #[test]
    fn http_error_status_surfaces() {
        let url = spawn_one_shot("HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n".to_string());
        let spec = DiscoverySpec::Endpoint {
            url,
            requires_auth: false,
            list_selector: "data".into(),
        };
        assert_eq!(
            fetch_models(&spec, AuthStyle::ApiKey, None),
            Err(DiscoveryError::HttpStatus(403))
        );
    }

    #[test]
    fn malformed_json_and_missing_selector_refused() {
        let url = spawn_one_shot("HTTP/1.1 200 OK\r\n\r\n{not json".to_string());
        let spec = DiscoverySpec::Endpoint {
            url,
            requires_auth: false,
            list_selector: "data".into(),
        };
        assert!(matches!(
            fetch_models(&spec, AuthStyle::ApiKey, None),
            Err(DiscoveryError::InvalidResponse(_))
        ));

        let url = spawn_one_shot("HTTP/1.1 200 OK\r\n\r\n{\"wrong\":[]}".to_string());
        let spec = DiscoverySpec::Endpoint {
            url,
            requires_auth: false,
            list_selector: "data".into(),
        };
        assert!(matches!(
            fetch_models(&spec, AuthStyle::ApiKey, None),
            Err(DiscoveryError::InvalidResponse(_))
        ));
    }

    #[test]
    fn non_array_selector_refused() {
        let url = spawn_one_shot("HTTP/1.1 200 OK\r\n\r\n{\"data\":{\"id\":\"x\"}}".to_string());
        let spec = DiscoverySpec::Endpoint {
            url,
            requires_auth: false,
            list_selector: "data".into(),
        };
        assert!(matches!(
            fetch_models(&spec, AuthStyle::ApiKey, None),
            Err(DiscoveryError::InvalidResponse(_))
        ));
    }

    #[test]
    fn auth_headers_use_documented_style() {
        // The mock echoes headers back in the body for assertion.
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = vec![0u8; 8192];
            let n = stream.read(&mut buf).expect("read");
            let req = String::from_utf8_lossy(&buf[..n]).to_string();
            // Strip control characters so the echo is valid JSON.
            let sanitized: String = req
                .chars()
                .map(|c| if c.is_control() { ' ' } else { c })
                .collect();
            let body = format!(
                "{{\"data\":[{{\"id\":\"m\"}}],\"echo\":\"{}\"}}",
                sanitized.replace('"', "'")
            );
            let resp = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{body}");
            let _ = stream.write_all(resp.as_bytes());
        });
        let url = format!("http://{addr}/models");
        let spec = DiscoverySpec::Endpoint {
            url,
            requires_auth: true,
            list_selector: "data".into(),
        };

        let models = fetch_models(&spec, AuthStyle::ApiKey, Some("sk-fake-key")).expect("fetch");
        assert_eq!(models[0].id, "m");
        handle.join().expect("server thread");
    }

    #[test]
    fn transport_errors_strip_query_strings() {
        let msg = "connect failed to https://evil.example.com/?leak=secret&x=1";
        let red = redact_transport(msg);
        assert!(!red.contains("leak=secret"));
    }
}
