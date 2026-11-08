//! Connection / credential test ("Test connection").
//!
//! Save & Apply only proves the FILE was written, which the owner reasonably read
//! as "my key works" — it does not. This module answers the real question by
//! making one authenticated request to the provider and classifying the answer.
//!
//! Endpoint choice (verified against provider docs, never guessed):
//! - **OpenRouter** — `GET https://openrouter.ai/api/v1/key` requires the Bearer
//!   token (openrouter.ai/docs/api/api-reference/api-keys/get-current-api-key),
//!   so a 200 proves the key. Its `/models` endpoint is public and would prove
//!   nothing.
//! - **Anthropic / DeepSeek / NVIDIA NIM** — their documented catalog endpoints
//!   require the credential, so a 200 there proves the key too.
//! - **Z.AI / Custom** — no documented key-check endpoint ⇒ the test reports
//!   "unsupported" instead of inventing one (rule 9).
//!
//! The request carries the same hard policy as discovery: HTTPS only, redirects
//! disabled (a 3xx must not forward the credential), proxy from the environment,
//! OS trust store, 10 s timeout.

use crate::provider_catalog::{AuthStyle, DiscoverySpec, ProviderTemplate};
use serde::Serialize;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTest {
    pub provider_id: String,
    pub ok: bool,
    pub status: Option<u16>,
    /// Human sentence for the UI. Never contains the credential.
    pub message: String,
    pub endpoint: String,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TestError {
    /// No documented key-check endpoint for this provider.
    Unsupported(String),
    /// Nothing stored to test.
    MissingCredential,
    /// Request could not be made (DNS, connect, timeout) — message pre-redacted.
    Transport(String),
}

impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(p) => write!(
                f,
                "there is no documented key-check endpoint for {p}; the key cannot be verified from here"
            ),
            Self::MissingCredential => write!(
                f,
                "no credential is stored for this provider — save one first"
            ),
            Self::Transport(m) => write!(f, "connection failed: {m}"),
        }
    }
}

/// The endpoint that proves a key works, or `None` when none is documented.
pub fn key_check_target(template: &ProviderTemplate) -> Option<(String, AuthStyle)> {
    match template.id.as_str() {
        // `/models` is public here, so it would "pass" with any key.
        "openrouter" => Some((
            "https://openrouter.ai/api/v1/key".to_string(),
            AuthStyle::BearerToken,
        )),
        // Z.AI has no documented catalog/key endpoint in our registry; custom
        // endpoints are unknown by definition.
        "zai" | "custom" => None,
        _ => match &template.discovery {
            DiscoverySpec::Endpoint {
                url,
                requires_auth: true,
                ..
            } => Some((
                url.clone(),
                template.discovery_auth.unwrap_or(template.auth),
            )),
            _ => None,
        },
    }
}

fn agent() -> ureq::Agent {
    // Same posture as discovery (G2/G10): no redirects, OS trust store, proxy
    // from the environment, bounded time.
    let tls = ureq::tls::TlsConfig::builder()
        .root_certs(ureq::tls::RootCerts::PlatformVerifier)
        .build();
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .max_redirects(0)
        .max_redirects_will_error(true)
        .redirect_auth_headers(ureq::config::RedirectAuthHeaders::Never)
        .proxy(ureq::Proxy::try_from_env())
        .tls_config(tls)
        .build();
    config.into()
}

fn interpret(status: u16, provider_id: &str, endpoint: &str, latency_ms: u64) -> ConnectionTest {
    let (ok, message) = match status {
        200..=299 => (
            true,
            "key accepted by the provider".to_string(),
        ),
        401 => (
            false,
            "the provider rejected this key (401 Unauthorized) — it is wrong, expired, or revoked"
                .to_string(),
        ),
        402 => (
            false,
            "the key is valid but the account has no credit (402 Payment Required)".to_string(),
        ),
        403 => (
            false,
            "the provider accepted the key but refused access (403 Forbidden) — check the key's \
             permissions or region"
                .to_string(),
        ),
        404 => (
            false,
            "the provider answered 404 — the endpoint path is wrong for this key (check the base URL)"
                .to_string(),
        ),
        429 => (
            true,
            "the key is accepted; the account is rate limited right now (429)".to_string(),
        ),
        500..=599 => (
            false,
            format!("the provider is failing on its side ({status}) — the key was not judged"),
        ),
        other => (
            false,
            format!("unexpected response from the provider ({other})"),
        ),
    };
    ConnectionTest {
        provider_id: provider_id.to_string(),
        ok,
        status: Some(status),
        message,
        endpoint: endpoint.to_string(),
        latency_ms,
    }
}

/// Perform the check. `credential` is the secret to test (usually the stored one).
pub fn test_connection(
    template: &ProviderTemplate,
    credential: &str,
) -> Result<ConnectionTest, TestError> {
    let (url, auth) =
        key_check_target(template).ok_or_else(|| TestError::Unsupported(template.name.clone()))?;
    if credential.is_empty() {
        return Err(TestError::MissingCredential);
    }

    let started = Instant::now();
    let mut req = agent().get(&url);
    req = match auth {
        AuthStyle::ApiKey => req.header("x-api-key", credential),
        AuthStyle::BearerToken => req.header("authorization", &format!("Bearer {credential}")),
    };
    if template.id == "anthropic" {
        // Required by the Anthropic API on every request.
        req = req.header("anthropic-version", "2023-06-01");
    }

    match req.call() {
        Ok(resp) => Ok(interpret(
            resp.status().as_u16(),
            &template.id,
            &url,
            started.elapsed().as_millis() as u64,
        )),
        Err(ureq::Error::StatusCode(code)) => Ok(interpret(
            code,
            &template.id,
            &url,
            started.elapsed().as_millis() as u64,
        )),
        Err(ureq::Error::Timeout(_)) => Err(TestError::Transport(
            "no answer within 10 s (network blocked, or the provider is slow)".to_string(),
        )),
        Err(e) => Err(TestError::Transport(crate::redact::strip_secret_shaped(
            &e.to_string(),
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presets::load_builtin_presets;
    use crate::provider_catalog::ProviderTemplate;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    fn find_template(id: &str) -> Option<ProviderTemplate> {
        load_builtin_presets().into_iter().find(|t| t.id == id)
    }

    fn template(id: &str) -> ProviderTemplate {
        find_template(id).expect("preset present")
    }

    fn serve_once(status: u16, body: &str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        let body = body.to_string();
        std::thread::spawn(move || {
            if let Ok((mut sock, _)) = listener.accept() {
                let mut buf = [0u8; 4096];
                let _ = sock.read(&mut buf);
                let resp = format!(
                    "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = sock.write_all(resp.as_bytes());
            }
        });
        format!("http://{addr}/key")
    }

    fn template_with_url(id: &str, url: &str) -> ProviderTemplate {
        let mut t = template(id);
        t.discovery = DiscoverySpec::Endpoint {
            url: url.to_string(),
            requires_auth: true,
            list_selector: "data".into(),
        };
        t
    }

    #[test]
    fn a_200_means_the_key_works() {
        let t = template_with_url("deepseek", &serve_once(200, "{\"data\":[]}"));
        let out = test_connection(&t, "sk-ant-FAKE-test").expect("result");
        assert!(out.ok, "{out:?}");
        assert_eq!(out.status, Some(200));
        assert!(out.message.contains("accepted"));
    }

    #[test]
    fn a_401_is_named_as_a_key_problem() {
        let t = template_with_url("deepseek", &serve_once(401, "{\"error\":\"nope\"}"));
        let out = test_connection(&t, "sk-ant-FAKE-bad").expect("result");
        assert!(!out.ok);
        assert!(out.message.contains("401"));
        assert!(out.message.contains("wrong, expired, or revoked"));
    }

    #[test]
    fn a_403_and_404_are_distinguished_from_a_bad_key() {
        let t403 = template_with_url("deepseek", &serve_once(403, ""));
        let out = test_connection(&t403, "k").expect("result");
        assert!(out.message.contains("403"));
        let t404 = template_with_url("deepseek", &serve_once(404, ""));
        let out = test_connection(&t404, "k").expect("result");
        assert!(out.message.contains("base URL"));
    }

    #[test]
    fn a_rate_limited_account_still_has_a_valid_key() {
        let t = template_with_url("deepseek", &serve_once(429, ""));
        let out = test_connection(&t, "k").expect("result");
        assert!(out.ok, "429 means the key was accepted: {out:?}");
    }

    #[test]
    fn providers_without_a_documented_endpoint_are_not_guessed() {
        // Z.AI ships as a preset but has no documented key-check endpoint.
        let zai = template("zai");
        assert!(
            key_check_target(&zai).is_none(),
            "zai must not invent an endpoint"
        );
        // A custom endpoint is unknown by definition (no preset exists for it).
        let mut custom = template("zai");
        custom.id = "custom".to_string();
        custom.name = "Custom".to_string();
        custom.discovery = DiscoverySpec::None;
        assert!(key_check_target(&custom).is_none());
        assert!(matches!(
            test_connection(&custom, "sk-ant-FAKE-x"),
            Err(TestError::Unsupported(_))
        ));
    }

    #[test]
    fn openrouter_is_checked_against_the_key_endpoint_not_the_public_catalog() {
        let t = template("openrouter");
        let (url, auth) = key_check_target(&t).expect("target");
        assert_eq!(url, "https://openrouter.ai/api/v1/key");
        assert_eq!(auth, AuthStyle::BearerToken);
    }

    #[test]
    fn an_empty_credential_is_refused_before_any_request() {
        let t = template("deepseek");
        assert_eq!(test_connection(&t, ""), Err(TestError::MissingCredential));
    }

    #[test]
    fn the_credential_is_sent_as_the_configured_header_style() {
        // Record the request line + headers the server actually receives.
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            if let Ok((mut sock, _)) = listener.accept() {
                let mut buf = [0u8; 4096];
                let n = sock.read(&mut buf).unwrap_or(0);
                tx.send(String::from_utf8_lossy(&buf[..n]).to_string()).ok();
                let _ = sock.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
                );
            }
        });
        let t = template_with_url("deepseek", &format!("http://{addr}/models"));
        let _ = test_connection(&t, "sk-ant-FAKE-header-check").expect("result");
        let seen = rx.recv_timeout(Duration::from_secs(5)).expect("request");
        assert!(
            seen.to_lowercase()
                .contains("authorization: bearer sk-ant-fake-header-check"),
            "deepseek catalog auth is Bearer; saw: {seen}"
        );
    }
}
