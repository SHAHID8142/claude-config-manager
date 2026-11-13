//! V3-03: regression baseline for the defects confirmed in `docs/V3/V3-02-defects.md`.
//!
//! These tests deliberately describe the *current, defective* behaviour, so the
//! defect is pinned by an executable assertion instead of a paragraph. When a
//! later V3 task fixes the cause, the assertion here is inverted in the same
//! commit — an unfixed defect can therefore never look fixed by accident.
//!
//! Currently covered here:
//!   §1  the proxy lives and dies with whoever created the handle (the GUI)
//!   §2  there is no request/response translation for OpenAI-only providers
//!   §6  a routing target cannot be saved with another provider's credential
//!   §7  CI is a billing failure upstream, not a code failure (documented only)

use claude_config_manager_lib::credential_vault::CredentialVault;
use claude_config_manager_lib::proxy;
use claude_config_manager_lib::routing_config::{self, RouteTarget, RoutingConfig};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

static N: AtomicU32 = AtomicU32::new(0);

fn temp_home(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ccm-v3-{}-{}-{name}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("home");
    dir
}

fn get(port: u16, path: &str) -> Option<String> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n"
    )
    .ok()?;
    let mut buf = String::new();
    stream.read_to_string(&mut buf).ok()?;
    Some(buf)
}

/// §1 — the defect, stated positively: while the owner of the handle lives, the
/// proxy answers; the moment the handle is dropped the port stops accepting.
/// Phase B must make the *first* half true for a process that outlives the GUI.
#[test]
fn proxy_port_lives_exactly_as_long_as_the_in_process_handle() {
    let home = temp_home("lifetime");
    let handle = proxy::start(home).expect("proxy starts");
    let port = handle.port;

    let alive = get(port, "/healthz").expect("healthz while the handle is alive");
    assert!(alive.contains("200"), "expected 200, got: {alive}");

    drop(handle);

    // A dropped handle must stop accepting. Retry briefly: the listener closes
    // asynchronously when the accept thread returns.
    let mut refused = false;
    for _ in 0..40 {
        if get(port, "/healthz").is_none() {
            refused = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    assert!(
        refused,
        "V3-02 §1 baseline: the proxy is bound to the creating process, so it \
         cannot outlive it. If this fails, a detached helper already exists — \
         update docs/V3/V3-02-defects.md §1 and this test."
    );
}

/// §2 — the proxy only speaks the Anthropic Messages API. An OpenAI-only
/// upstream (chat/completions) is rejected by the provider, and nothing in the
/// code path translates it. This test proves the *absence* of translation by
/// showing the request body is forwarded byte-for-byte.
#[test]
fn openai_only_upstream_receives_anthropic_body_without_translation() {
    let home = temp_home("no-bridge");
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("upstream");
    let addr = listener.local_addr().expect("addr");
    let base = format!("http://{addr}");

    let seen = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let seen_writer = seen.clone();
    let server = std::thread::spawn(move || {
        let (mut sock, _) = listener.accept().expect("accept");
        // Read until the request is complete (headers + Content-Length body).
        // A single read can return headers before the body lands when the
        // machine is under load, which made this fixture flaky.
        sock.set_read_timeout(Some(Duration::from_secs(5))).ok();
        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            match sock.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    buf.extend_from_slice(&chunk[..n]);
                    let s = String::from_utf8_lossy(&buf);
                    if let Some(cl) = s
                        .to_lowercase()
                        .split("content-length:")
                        .nth(1)
                        .and_then(|rest| rest.split('\r').next()?.trim().parse::<usize>().ok())
                    {
                        let header_end = s.find("\r\n\r\n").map(|p| p + 4).unwrap_or(0);
                        if buf.len() >= header_end + cl {
                            break;
                        }
                    }
                }
                Err(_) => break,
            }
        }
        *seen_writer.lock().unwrap() = String::from_utf8_lossy(&buf).to_string();
        let body = "{\"error\":{\"message\":\"unknown endpoint\"}}";
        let _ = write!(
            sock,
            "HTTP/1.1 404 Not Found\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            body.len(),
            body
        );
    });

    let vault = CredentialVault::for_home(&home);
    vault
        .add("nvidia", "nvidia-1", "sk-fixture-nvidia")
        .expect("store credential");

    let cfg = RoutingConfig {
        version: 1,
        enabled: true,
        targets: vec![RouteTarget {
            id: "nvidia-1".into(),
            provider_id: "nvidia".into(),
            credential_id: Some("nvidia-1".into()),
            model: Some("meta/llama-3.1-70b-instruct".into()),
            base_url: base.clone(),
            enabled: true,
        }],
    };
    routing_config::save(&home, &cfg).expect("save routing");

    let handle = proxy::start(home).expect("proxy starts");
    let port = handle.port;

    let mut sock = TcpStream::connect(("127.0.0.1", port)).expect("connect proxy");
    sock.set_read_timeout(Some(Duration::from_secs(5))).ok();
    let body = r#"{"model":"claude-sonnet-4-5","max_tokens":8,"messages":[{"role":"user","content":"hi"}]}"#;
    write!(
        sock,
        "POST /v1/messages HTTP/1.1\r\nHost: 127.0.0.1\r\nx-api-key: {}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        handle.token,
        body.len()
    )
    .expect("send");
    let mut out = String::new();
    let _ = sock.read_to_string(&mut out);

    server.join().expect("upstream thread");
    let upstream_request = seen.lock().unwrap().clone();

    // The model is rewritten (that is V2 routing), but the payload shape is
    // untouched: `messages` + `max_tokens`, no `chat/completions` translation.
    assert!(
        upstream_request.contains("meta/llama-3.1-70b-instruct"),
        "routing must rewrite the model id; got: {upstream_request}"
    );
    assert!(
        upstream_request.contains("\"messages\"") && upstream_request.contains("\"max_tokens\""),
        "V3-02 §2 baseline: the Anthropic body is forwarded verbatim — no OpenAI translation exists"
    );
    assert!(
        !upstream_request.to_lowercase().contains("chat/completions"),
        "nothing rewrites the path to an OpenAI endpoint"
    );

    // And the caller sees the upstream's rejection, not a fabricated success.
    assert!(
        out.contains("404") || out.contains("unknown endpoint") || out.is_empty(),
        "unexpected proxy response: {out}"
    );
}

/// §6 — fixed in V3-20: a target may only use a credential stored for its own
/// provider. A hand-edited `routing.json` can no longer pair provider A with
/// provider B's key, which would have sent B's key to A's endpoint.
#[test]
fn cross_provider_credential_is_never_resolved() {
    let home = temp_home("mismatch");
    let vault = CredentialVault::for_home(&home);
    vault
        .add("openrouter", "openrouter-1", "sk-fixture-openrouter")
        .expect("store credential");

    let cfg = RoutingConfig {
        version: 1,
        enabled: true,
        targets: vec![RouteTarget {
            id: "t1".into(),
            provider_id: "deepseek".into(),
            credential_id: Some("openrouter-1".into()),
            model: None,
            base_url: "https://api.deepseek.com/anthropic".into(),
            enabled: true,
        }],
    };
    routing_config::save(&home, &cfg).expect("shape is valid; pairing is checked at resolve time");
    let loaded = routing_config::load(&home).expect("reload");
    assert_eq!(
        proxy::resolve_secret(&home, &loaded.targets[0]),
        None,
        "provider mismatch must resolve to no secret, not to another provider's key"
    );

    // The same credential still works for its own provider.
    let own = RouteTarget {
        id: "t2".into(),
        provider_id: "openrouter".into(),
        credential_id: Some("openrouter-1".into()),
        model: None,
        base_url: "https://openrouter.ai/api".into(),
        enabled: true,
    };
    assert_eq!(
        proxy::resolve_secret(&home, &own).as_deref(),
        Some("sk-fixture-openrouter")
    );
}

/// V3-20: a disabled credential is never handed to a target, even when a stale
/// `routing.json` still names it.
#[test]
fn disabled_credential_is_never_routed_to() {
    let home = temp_home("disabled");
    let vault = CredentialVault::for_home(&home);
    let rec = vault
        .add("openrouter", "openrouter-1", "sk-fixture-openrouter")
        .expect("store");
    assert!(vault.set_enabled(&rec.id, false).expect("disable"));

    let target = RouteTarget {
        id: "t1".into(),
        provider_id: "openrouter".into(),
        credential_id: Some("openrouter-1".into()),
        model: None,
        base_url: "https://openrouter.ai/api".into(),
        enabled: true,
    };
    assert_eq!(proxy::resolve_secret(&home, &target), None);
}

/// V3-13/V3-16 — capability protection: an OpenAI-only target is refused with an
/// explicit compatibility error before any request is sent, and the message says
/// what is missing. Nothing is silently translated or dropped.
#[test]
fn openai_only_target_is_refused_with_a_bridge_message() {
    use claude_config_manager_lib::proxy::{Adapter, adapter_for, bridge_requirement_message};

    assert_eq!(adapter_for("nvidia-nim"), Some(Adapter::OpenAiUnbridged));
    assert_eq!(
        adapter_for("openrouter"),
        Some(Adapter::AnthropicCompatible)
    );
    assert_eq!(adapter_for("anthropic"), Some(Adapter::AnthropicNative));
    assert_eq!(adapter_for("not-a-provider"), None);

    let msg = bridge_requirement_message("nvidia-nim");
    assert!(
        msg.contains("Chat Completions"),
        "says which protocol: {msg}"
    );
    assert!(msg.contains("bridge"), "says what is missing: {msg}");
    assert!(msg.contains("not sent"), "says nothing was sent: {msg}");
}

/// V3-16 — over a real socket: an OpenAI-only chain answers 501 with a
/// compatibility error, and the upstream is never contacted.
#[test]
fn openai_only_chain_returns_501_without_touching_the_upstream() {
    use std::io::{Read, Write};
    use std::net::TcpStream;

    let home = temp_home("bridge-501");
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("upstream");
    let addr = listener.local_addr().expect("addr");
    // If the proxy ever forwards, this accept() would succeed.
    listener.set_nonblocking(true).expect("nonblocking");

    let vault = CredentialVault::for_home(&home);
    vault
        .add("nvidia-nim", "nvidia-1", "sk-fixture-nvidia")
        .expect("store");

    let cfg = RoutingConfig {
        version: 1,
        enabled: true,
        targets: vec![RouteTarget {
            id: "nvidia-1".into(),
            provider_id: "nvidia-nim".into(),
            credential_id: Some("nvidia-1".into()),
            model: Some("meta/llama-3.1-70b-instruct".into()),
            base_url: format!("http://{addr}"),
            enabled: true,
        }],
    };
    routing_config::save(&home, &cfg).expect("save routing");

    let handle = proxy::start(home).expect("proxy");
    let mut sock = TcpStream::connect(("127.0.0.1", handle.port)).expect("connect");
    sock.set_read_timeout(Some(Duration::from_secs(5))).ok();
    let body = r#"{"model":"claude-sonnet-4-5","max_tokens":8,"messages":[{"role":"user","content":"hi"}]}"#;
    write!(
        sock,
        "POST /v1/messages HTTP/1.1\r\nHost: 127.0.0.1\r\nx-api-key: {}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        handle.token,
        body.len()
    )
    .expect("send");
    let mut out = String::new();
    let _ = sock.read_to_string(&mut out);

    assert!(out.contains("501"), "expected 501, got: {out}");
    assert!(out.contains("compatibility_error"), "got: {out}");
    assert!(out.contains("Chat Completions"), "got: {out}");
    assert!(
        listener.accept().is_err(),
        "the OpenAI-only upstream must never be contacted"
    );
}
