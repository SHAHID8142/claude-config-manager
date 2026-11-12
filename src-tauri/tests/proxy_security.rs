//! V3-32 — the proxy's own attack surface, over real sockets.
//!
//! Every test here talks raw HTTP to a listening proxy and asserts what comes
//! back. Nothing is mocked: these are the bytes a local process could send.

use claude_config_manager_lib::credential_vault::CredentialVault;
use claude_config_manager_lib::proxy;
use claude_config_manager_lib::routing_config::{self, RouteTarget, RoutingConfig};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

static N: AtomicU32 = AtomicU32::new(0);

fn temp_home(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ccm-v3sec-{}-{}-{name}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("home");
    dir
}

/// A proxy with one target whose upstream is a closed port: every request that
/// gets past validation fails at the network step, which is fine — these tests
/// assert the validation boundary itself.
fn proxy_with_dead_upstream(name: &str) -> (std::path::PathBuf, proxy::ProxyHandle) {
    let home = temp_home(name);
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("reserve port");
    let addr = listener.local_addr().expect("addr");
    drop(listener); // port is now closed

    let vault = CredentialVault::for_home(&home);
    vault
        .add("openrouter", "openrouter-1", "sk-fixture-secret-value")
        .expect("store");

    let cfg = RoutingConfig {
        version: 1,
        enabled: true,
        targets: vec![RouteTarget {
            id: "or-1".into(),
            provider_id: "openrouter".into(),
            credential_id: Some("openrouter-1".into()),
            model: None,
            base_url: format!("http://{addr}"),
            enabled: true,
        }],
    };
    routing_config::save(&home, &cfg).expect("routing");
    let handle = proxy::start(home.clone()).expect("proxy");
    (home, handle)
}

fn send(port: u16, raw: &str) -> String {
    let mut sock = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    sock.set_read_timeout(Some(Duration::from_secs(10))).ok();
    sock.write_all(raw.as_bytes()).expect("write");
    let mut out = String::new();
    let _ = sock.read_to_string(&mut out);
    out
}

fn head(status: &str) -> String {
    format!("statuses: {status}")
}

#[test]
fn a_request_without_the_token_is_401_and_never_reaches_the_upstream() {
    let (_home, handle) = proxy_with_dead_upstream("no-token");
    let out = send(
        handle.port,
        "POST /v1/messages HTTP/1.1\r\nHost: 127.0.0.1\r\ncontent-length: 2\r\nconnection: close\r\n\r\n{}",
    );
    assert!(out.contains("401"), "{}", head(&out));
    assert!(
        !out.contains("sk-fixture-secret-value"),
        "no secret may echo back"
    );
}

#[test]
fn a_wrong_token_is_401_and_an_empty_token_is_401() {
    let (_home, handle) = proxy_with_dead_upstream("wrong-token");
    for header in [
        "x-api-key: not-the-token\r\n",
        "authorization: Bearer nope\r\n",
        "",
    ] {
        let raw = format!(
            "POST /v1/messages HTTP/1.1\r\nHost: 127.0.0.1\r\n{header}content-length: 2\r\nconnection: close\r\n\r\n{{}}"
        );
        let out = send(handle.port, &raw);
        assert!(
            out.contains("401"),
            "header {header:?} gave: {}",
            head(&out)
        );
    }
}

#[test]
fn a_malformed_request_line_is_400() {
    let (_home, handle) = proxy_with_dead_upstream("malformed");
    let out = send(handle.port, "GARBAGE\r\n\r\n");
    assert!(out.contains("400"), "{}", head(&out));
}

#[test]
fn oversized_headers_are_431() {
    let (_home, handle) = proxy_with_dead_upstream("big-headers");
    let mut raw = String::from("GET /healthz HTTP/1.1\r\nHost: 127.0.0.1\r\n");
    for i in 0..2000 {
        raw.push_str(&format!("x-pad-{i}: {}\r\n", "a".repeat(64)));
    }
    raw.push_str("\r\n");
    let out = send(handle.port, &raw);
    assert!(out.contains("431"), "{}", head(&out));
}

#[test]
fn a_body_larger_than_the_limit_is_413_before_it_is_buffered() {
    let (_home, handle) = proxy_with_dead_upstream("big-body");
    let raw = format!(
        "POST /v1/messages HTTP/1.1\r\nHost: 127.0.0.1\r\nx-api-key: {}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
        handle.token,
        16 * 1024 * 1024 + 1
    );
    let out = send(handle.port, &raw);
    assert!(out.contains("413"), "{}", head(&out));
}

#[test]
fn only_post_v1_messages_is_proxied() {
    let (_home, handle) = proxy_with_dead_upstream("paths");
    for path in ["/", "/v1/models", "/admin", "/v1/messages/../etc/passwd"] {
        let raw = format!(
            "POST {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nx-api-key: {}\r\ncontent-length: 2\r\nconnection: close\r\n\r\n{{}}",
            handle.token
        );
        let out = send(handle.port, &raw);
        assert!(out.contains("404"), "path {path} gave: {}", head(&out));
    }
}

#[test]
fn a_crash_in_one_request_does_not_take_the_listener_down() {
    let (_home, handle) = proxy_with_dead_upstream("resilience");
    // A truncated body, then a valid request: the listener must survive both.
    let _ = send(
        handle.port,
        &format!(
            "POST /v1/messages HTTP/1.1\r\nHost: 127.0.0.1\r\nx-api-key: {}\r\ncontent-length: 100\r\nconnection: close\r\n\r\nshort",
            handle.token
        ),
    );
    let out = send(
        handle.port,
        "GET /healthz HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
    );
    assert!(out.contains("200"), "listener died: {}", head(&out));
}

#[test]
fn the_health_endpoint_never_contains_a_credential_or_a_token() {
    let (_home, handle) = proxy_with_dead_upstream("health");
    let out = send(
        handle.port,
        "GET /healthz HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
    );
    assert!(out.contains("200"));
    assert!(!out.contains("sk-fixture-secret-value"));
    assert!(
        !out.contains(&handle.token),
        "the bearer token is not public"
    );
}
