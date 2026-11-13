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
    /// Context window the catalog declares for this model, when published
    /// (OpenRouter: `context_length`). Sourced, never invented (F12) — the
    /// UI offers it as a one-click value for `CLAUDE_CODE_MAX_CONTEXT_TOKENS`.
    pub context_window: Option<u64>,
    /// Always `unknown` at discovery time — capabilities are never invented.
    pub capability_status: &'static str,
    /// Provenance of this record: the verified catalog URL it was read from
    /// (never a guess, never a synthesized capability source).
    pub source: String,
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
    // T60: same rules as `classify_endpoint` — control/whitespace characters,
    // embedded credentials and wildcard hosts are refused before parsing can
    // normalise them into something else.
    if url.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(DiscoveryError::InvalidEndpoint(
            "URL must not contain whitespace or control characters".into(),
        ));
    }
    let parsed =
        url::Url::parse(url).map_err(|e| DiscoveryError::InvalidEndpoint(e.to_string()))?;
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(DiscoveryError::InvalidEndpoint(
            "URL must not embed credentials".into(),
        ));
    }
    let host = parsed.host_str().unwrap_or_default();
    if matches!(host, "0.0.0.0" | "::" | "[::]") || host.starts_with("169.254.") {
        return Err(DiscoveryError::InvalidEndpoint(
            "wildcard or link-local host is not a usable endpoint".into(),
        ));
    }
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
        // T59: whitespace-only ids are unusable — they would render as a blank
        // row and then be rejected at apply time.
        let id = id.trim();
        if id.is_empty() || id.len() > 256 || id.chars().any(|c| c.is_control()) {
            continue;
        }
        // Catalog-declared context window (OpenRouter `context_length`).
        // Accepted range mirrors config_merge's CLAUDE_CODE_MAX_CONTEXT_TOKENS
        // validation (1,000–10,000,000) so a suggested value can never be
        // rejected at apply time.
        let context_window = obj
            .get("context_length")
            .and_then(|v| v.as_u64())
            .filter(|n| (1_000..=10_000_000).contains(n));
        out.push(ModelEntry {
            id: id.to_string(),
            display_name: obj
                .get("display_name")
                .or_else(|| obj.get("name"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            context_window,
            // Always `unknown` at discovery time — capabilities are never
            // invented (F12).
            capability_status: "unknown",
            // Provenance is the verified template endpoint, not the payload's
            // self-description (a hostile catalog cannot claim its own source).
            source: url.clone(),
        });
    }
    Ok(out)
}

fn http_get(
    url: &str,
    auth_style: AuthStyle,
    credential: Option<&str>,
) -> Result<String, DiscoveryError> {
    // G10: honour the standard proxy environment variables (including NO_PROXY)
    // rather than pretending corporate networks do not exist.
    http_get_with_proxy(url, auth_style, credential, ureq::Proxy::try_from_env())
}

fn http_get_with_proxy(
    url: &str,
    auth_style: AuthStyle,
    credential: Option<&str>,
    proxy: Option<ureq::Proxy>,
) -> Result<String, DiscoveryError> {
    // Agent with a global timeout (ureq v3 config API).
    //
    // Redirects are DISABLED deliberately (G2). `ureq`'s defaults happen to be
    // safe, but relying on a dependency default that no test asserts is how
    // credential leaks get reintroduced on an upgrade. A catalog URL that
    // redirects is not something we need: refuse it and let the user fix the
    // endpoint. `redirect_auth_headers(Never)` is kept as defence in depth in
    // case the redirect cap is ever relaxed.
    let config = ureq::config::Config::builder()
        .timeout_global(Some(DISCOVERY_TIMEOUT))
        .max_redirects(0)
        .max_redirects_will_error(true)
        .redirect_auth_headers(ureq::config::RedirectAuthHeaders::Never)
        .proxy(proxy)
        .tls_config(
            // Trust the OS store: a certificate installed by the user's
            // organisation (MDM) must work, and TLS verification stays ON.
            ureq::tls::TlsConfig::builder()
                .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                .build(),
        )
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

    // Redirects are not followed (`max_redirects(0)`), so a 3xx arrives here as
    // an ordinary response. Report it as such instead of failing later with a
    // misleading "response unusable" parse error.
    if (300..400).contains(&response.status().as_u16()) {
        return Err(DiscoveryError::HttpStatus(response.status().as_u16()));
    }

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
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

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
        // Provenance is the verified template endpoint for every entry (T42).
        let expected_source = match &spec {
            DiscoverySpec::Endpoint { url, .. } => url.clone(),
            DiscoverySpec::None => panic!("endpoint spec expected"),
        };
        for model in &models {
            assert_eq!(model.source, expected_source);
            assert_eq!(model.capability_status, "unknown");
        }
    }

    #[test]
    fn source_is_template_url_not_payload_claim() {
        // A hostile catalog cannot set its own provenance: the `source` key in
        // the body is ignored and the verified template URL is reported.
        let url = spawn_one_shot(
            "HTTP/1.1 200 OK\r\n\r\n{\"data\":[{\"id\":\"m\",\"source\":\"https://evil.example.com\",\"context_window\":999999}]}"
                .to_string(),
        );
        let spec = DiscoverySpec::Endpoint {
            url: url.clone(),
            requires_auth: false,
            list_selector: "data".into(),
        };
        let models = fetch_models(&spec, AuthStyle::ApiKey, None).expect("fetch");
        assert_eq!(
            models[0].source, url,
            "source must be the verified endpoint"
        );
        assert!(!models[0].source.contains("evil.example.com"));
        // No capability fields are carried at all — the wire shape is fixed.
        let json = serde_json::to_value(&models[0]).expect("serialize");
        assert_eq!(json["capabilityStatus"], "unknown");
        // Absent in the catalog -> serialized as null (never a fabricated
        // number).
        assert!(json["contextWindow"].is_null());
        assert_eq!(
            json.as_object().expect("object").len(),
            5,
            "id/displayName/contextWindow/capabilityStatus/source only"
        );
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

    /// Capture the raw request line + headers our client sends for a given
    /// credential style against a loopback fixture.
    fn captured_request(auth: AuthStyle, credential: &str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let addr = listener.local_addr().expect("addr");
        let (tx, rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = vec![0u8; 8192];
            let n = stream.read(&mut buf).expect("read");
            let _ = tx.send(String::from_utf8_lossy(&buf[..n]).to_string());
            let body = "{\"data\":[{\"id\":\"m\"}]}";
            let resp = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{body}");
            let _ = stream.write_all(resp.as_bytes());
        });

        let spec = DiscoverySpec::Endpoint {
            url: format!("http://{addr}/models"),
            requires_auth: true,
            list_selector: "data".into(),
        };
        let models = fetch_models(&spec, auth, Some(credential)).expect("fetch");
        assert_eq!(models[0].id, "m");

        let req = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("request captured");
        handle.join().expect("server thread");
        req.to_lowercase()
    }

    /// G10: an explicit proxy is actually used (the request reaches the proxy and
    /// never the origin). Proves the proxy plumbing works without depending on
    /// process-wide environment variables, which tests must not mutate.
    #[test]
    fn requests_go_through_the_configured_proxy() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::sync::mpsc;

        let (tx, rx) = mpsc::channel::<String>();
        let proxy = TcpListener::bind("127.0.0.1:0").expect("bind proxy");
        let proxy_port = proxy.local_addr().expect("addr").port();
        std::thread::spawn(move || {
            for stream in proxy.incoming().take(1) {
                let mut stream = stream.expect("accept");
                // ureq tunnels even plain-HTTP origins through CONNECT, so the
                // fake proxy must complete the tunnel before seeing the request.
                let mut buf = [0u8; 512];
                let mut connect = String::new();
                while !connect.contains("\r\n\r\n") {
                    let n = stream.read(&mut buf).unwrap_or(0);
                    if n == 0 {
                        return;
                    }
                    connect.push_str(&String::from_utf8_lossy(&buf[..n]));
                }
                stream
                    .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
                    .ok();

                let mut request = String::new();
                while !request.contains("\r\n\r\n") {
                    let n = stream.read(&mut buf).unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    request.push_str(&String::from_utf8_lossy(&buf[..n]));
                }
                tx.send(format!("CONNECT {connect}\n----\n{request}")).ok();
                stream
                    .write_all(
                        b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 11\r\nconnection: close\r\n\r\n{\"data\":[]}",
                    )
                    .ok();
                stream.flush().ok();
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        });

        let proxy = ureq::Proxy::new(&format!("http://127.0.0.1:{proxy_port}")).expect("proxy url");
        let body = http_get_with_proxy(
            "http://origin.invalid/v1/models",
            AuthStyle::ApiKey,
            Some("sk-proxy-test"),
            Some(proxy),
        )
        .expect("proxied request");

        assert_eq!(body.trim(), r#"{"data":[]}"#);
        let seen = rx
            .recv_timeout(std::time::Duration::from_secs(1))
            .expect("proxy saw a request");
        assert!(
            seen.contains("CONNECT origin.invalid:80"),
            "proxy must be asked to tunnel to the origin, got: {seen}"
        );
        assert!(
            seen.contains("GET /v1/models"),
            "the tunnelled request must reach the proxy, got: {seen}"
        );
        // Credentials are sent to the endpoint through the proxy as configured —
        // the proxy itself does not add or strip them.
        assert!(seen.contains("x-api-key: sk-proxy-test"));
    }

    /// TLS verification must stay ON. Live (network) test: a self-signed
    /// certificate must be rejected, not silently accepted.
    #[test]
    #[ignore = "live network: requires internet access"]
    fn live_self_signed_certificate_is_refused() {
        let err = http_get("https://self-signed.badssl.com/", AuthStyle::ApiKey, None)
            .expect_err("a self-signed certificate must be refused");
        let msg = err.to_string().to_lowercase();
        assert!(
            msg.contains("certificate") || msg.contains("tls") || msg.contains("transport"),
            "expected a TLS failure, got: {err}"
        );
    }

    /// G5: an endless catalog response must be cut off at the cap instead of
    /// being read into memory. The fixture streams more than the cap and never
    /// closes, so a missing cap would hang or allocate unbounded.
    #[test]
    fn oversized_catalog_body_is_cut_off_at_the_cap() {
        use std::io::Write;
        use std::net::TcpListener;

        let server = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = server.local_addr().expect("addr").port();
        std::thread::spawn(move || {
            for stream in server.incoming().take(1) {
                let mut stream = stream.expect("accept");
                let mut buf = [0u8; 2048];
                let _ = std::io::Read::read(&mut stream, &mut buf);
                let _ = stream.write_all(
                    b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ntransfer-encoding: chunked\r\n\r\n",
                );
                // Stream repeatedly past the cap without finishing the body.
                let chunk = b"{\"data\":[";
                for _ in 0..(MAX_RESPONSE_BYTES / chunk.len() + 64) {
                    if stream
                        .write_all(format!("{:x}\r\n", chunk.len()).as_bytes())
                        .is_err()
                    {
                        return;
                    }
                    if stream.write_all(chunk).is_err() || stream.write_all(b"\r\n").is_err() {
                        return;
                    }
                }
            }
        });

        let err = http_get(
            &format!("http://127.0.0.1:{port}/v1/models"),
            AuthStyle::ApiKey,
            None,
        )
        .expect_err("an oversized body must be refused");
        assert!(
            err.to_string().contains("too large"),
            "expected a size error, got {err}"
        );
    }

    /// G2: a redirecting catalog must not cause the credential to be sent
    /// anywhere else — not even to a loopback port, standing in for "another
    /// host". The second server records any request it sees.
    #[test]
    fn a_redirecting_catalog_is_refused_and_the_credential_is_not_forwarded() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::sync::mpsc;

        let (tx, rx) = mpsc::channel::<(String, String)>();
        // Target server: records everything, answers 200.
        let target = TcpListener::bind("127.0.0.1:0").expect("bind target");
        let target_port = target.local_addr().expect("addr").port();
        std::thread::spawn(move || {
            for stream in target.incoming().take(1) {
                let mut stream = stream.expect("accept");
                let mut buf = [0u8; 4096];
                let n = stream.read(&mut buf).unwrap_or(0);
                let head = String::from_utf8_lossy(&buf[..n]).to_string();
                tx.send(("target".to_string(), head)).ok();
                stream
                    .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\n\r\n{}")
                    .ok();
            }
        });

        // Redirecting server: 302 → target.
        let redirect = TcpListener::bind("127.0.0.1:0").expect("bind redirect");
        let redirect_port = redirect.local_addr().expect("addr").port();
        std::thread::spawn(move || {
            for stream in redirect.incoming().take(1) {
                let mut stream = stream.expect("accept");
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf);
                let reply = format!(
                    "HTTP/1.1 302 Found\r\nlocation: http://127.0.0.1:{target_port}/v1/models\r\ncontent-length: 0\r\n\r\n"
                );
                stream.write_all(reply.as_bytes()).ok();
            }
        });

        let err = http_get(
            &format!("http://127.0.0.1:{redirect_port}/v1/models"),
            AuthStyle::BearerToken,
            Some("sk-secret-G2"),
        )
        .expect_err("a redirect must not be followed");
        assert!(
            matches!(err, DiscoveryError::HttpStatus(302)),
            "expected an explicit 302, got {err:?}"
        );

        // What matters most: the target was never contacted.
        let forwarded = rx
            .recv_timeout(std::time::Duration::from_millis(300))
            .map(|(_, head)| head)
            .unwrap_or_default();
        assert!(
            forwarded.is_empty(),
            "credential request was forwarded to the redirect target: {forwarded}"
        );
        assert!(
            !forwarded.contains("sk-secret-G2"),
            "credential leaked across the redirect"
        );
    }

    #[test]
    fn api_key_style_sends_x_api_key_header() {
        let req = captured_request(AuthStyle::ApiKey, "sk-fake-key");
        assert!(req.contains("x-api-key: sk-fake-key"), "request was: {req}");
        assert!(!req.contains("authorization:"), "request was: {req}");
    }

    #[test]
    fn bearer_style_sends_authorization_header() {
        // T44b: the style DeepSeek's /models catalog needs (docs-verified).
        let req = captured_request(AuthStyle::BearerToken, "sk-fake-key");
        assert!(
            req.contains("authorization: bearer sk-fake-key"),
            "request was: {req}"
        );
        assert!(!req.contains("x-api-key:"), "request was: {req}");
    }

    #[test]
    fn credential_is_never_sent_when_catalog_is_public() {
        // Independent of style: no credential means no auth header at all.
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        let (tx, rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = vec![0u8; 8192];
            let n = stream.read(&mut buf).expect("read");
            let _ = tx.send(String::from_utf8_lossy(&buf[..n]).to_string());
            let _ = stream.write_all(b"HTTP/1.1 200 OK\r\n\r\n{\"data\":[{\"id\":\"m\"}]}");
        });
        let spec = DiscoverySpec::Endpoint {
            url: format!("http://{addr}/models"),
            requires_auth: false,
            list_selector: "data".into(),
        };
        fetch_models(&spec, AuthStyle::BearerToken, None).expect("fetch");
        let req = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("captured")
            .to_lowercase();
        handle.join().expect("server thread");
        assert!(!req.contains("authorization:"), "request was: {req}");
        assert!(!req.contains("x-api-key:"), "request was: {req}");
    }

    #[test]
    fn transport_errors_strip_query_strings() {
        let msg = "connect failed to https://evil.example.com/?leak=secret&x=1";
        let red = redact_transport(msg);
        assert!(!red.contains("leak=secret"));
    }
}
