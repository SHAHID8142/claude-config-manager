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
    write!(stream, "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n").ok()?;
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
        let mut buf = [0u8; 8192];
        let n = sock.read(&mut buf).unwrap_or(0);
        *seen_writer.lock().unwrap() = String::from_utf8_lossy(&buf[..n]).to_string();
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

/// §6 — deeper than reported: neither `RoutingConfig::validate` nor
/// `proxy::resolve_secret` checks that a target's credential belongs to that
/// target's provider. A hand-edited `routing.json` can therefore pair provider A
/// with provider B's key, and the proxy will send B's key to A's endpoint.
/// The UI cannot create this (it filters by provider), so it is not reachable
/// from the app — but it is a real cross-provider credential leak for anyone who
/// edits the file. V3-20 must reject it in `resolve_secret`; this test then
/// inverts.
#[test]
fn defect_06_a_target_can_currently_carry_another_providers_key() {
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
            provider_id: "deepseek".into(),          // provider
            credential_id: Some("openrouter-1".into()), // …but another provider's key
            model: None,
            base_url: "https://api.deepseek.com/anthropic".into(),
            enabled: true,
        }],
    };

    // Current behaviour: accepted.
    routing_config::save(&home, &cfg).expect("V3-02 §6: validation does not pair credential and provider");
    let loaded = routing_config::load(&home).expect("reload");
    let leaked = proxy::resolve_secret(&home, &loaded.targets[0]).expect("secret resolves");
    assert_eq!(
        leaked, "sk-fixture-openrouter",
        "the proxy hands another provider's key to this target's endpoint — the defect V3-20 closes"
    );
}
