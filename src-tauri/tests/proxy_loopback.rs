//! V2-29/36/41: the loopback proxy against a real (scripted) upstream server.
//!
//! These tests bind real 127.0.0.1 sockets and speak raw HTTP on both sides, so
//! they exercise the same code path Claude Code would: auth, target order,
//! fallback, streaming passthrough, and no-fallback-on-request-errors.

use claude_config_manager_lib::credential_vault::CredentialVault;
use claude_config_manager_lib::proxy;
use claude_config_manager_lib::routing_config::{self, RouteTarget, RoutingConfig};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

static N: AtomicU32 = AtomicU32::new(0);

fn temp_home(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ccm-proxy-{}-{}-{name}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("home");
    dir
}

/// One scripted upstream: each accepted connection gets the next response.
struct Upstream {
    addr: String,
    hits: Arc<AtomicU32>,
    bodies: Arc<Mutex<Vec<String>>>,
}

fn spawn_upstream(responses: Vec<(u16, String)>) -> Upstream {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind upstream");
    let addr = format!("http://{}", listener.local_addr().expect("addr"));
    let hits = Arc::new(AtomicU32::new(0));
    let bodies = Arc::new(Mutex::new(Vec::new()));
    let hits_c = Arc::clone(&hits);
    let bodies_c = Arc::clone(&bodies);
    std::thread::spawn(move || {
        for (i, stream) in listener.incoming().enumerate() {
            let Ok(mut s) = stream else { break };
            hits_c.fetch_add(1, Ordering::SeqCst);
            let mut reader = BufReader::new(s.try_clone().expect("clone"));
            let mut line = String::new();
            let _ = reader.read_line(&mut line);
            let mut len = 0usize;
            loop {
                let mut h = String::new();
                if reader.read_line(&mut h).is_err() {
                    break;
                }
                if h.trim().is_empty() {
                    break;
                }
                if let Some((k, v)) = h.split_once(':')
                    && k.eq_ignore_ascii_case("content-length")
                {
                    len = v.trim().parse().unwrap_or(0);
                }
            }
            let mut body = vec![0u8; len];
            let _ = reader.read_exact(&mut body);
            bodies_c
                .lock()
                .expect("bodies")
                .push(String::from_utf8_lossy(&body).to_string());

            let (status, payload) = responses
                .get(i)
                .cloned()
                .unwrap_or((500, "exhausted".to_string()));
            let ctype = if payload.starts_with("event:") {
                "text/event-stream"
            } else {
                "application/json"
            };
            let head = format!(
                "HTTP/1.1 {status} X\r\ncontent-type: {ctype}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                payload.len()
            );
            let _ = s.write_all(head.as_bytes());
            let _ = s.write_all(payload.as_bytes());
            let _ = s.flush();
        }
    });
    Upstream { addr, hits, bodies }
}

fn target(id: &str, addr: &str, model: Option<&str>) -> RouteTarget {
    RouteTarget {
        id: id.into(),
        provider_id: "openrouter".into(),
        credential_id: Some("openrouter-1".into()),
        model: model.map(|m| m.to_string()),
        base_url: addr.into(),
        enabled: true,
    }
}

fn setup(name: &str, targets: Vec<RouteTarget>) -> (std::path::PathBuf, proxy::ProxyHandle) {
    let home = temp_home(name);
    let vault = CredentialVault::for_home(&home);
    vault
        .add("openrouter", "test", "sk-or-v1-fixture-key")
        .expect("store credential");
    routing_config::save(
        &home,
        &RoutingConfig {
            version: routing_config::ROUTING_VERSION,
            enabled: true,
            targets,
        },
    )
    .expect("save routing");
    let handle = proxy::start(home.clone()).expect("start proxy");
    (home, handle)
}

/// Raw client: returns (status line, full body).
fn call(port: u16, token: &str, body: &str) -> (String, String) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).expect("connect proxy");
    s.set_read_timeout(Some(Duration::from_secs(20)))
        .expect("timeout");
    let req = format!(
        "POST /v1/messages HTTP/1.1\r\nhost: 127.0.0.1\r\nauthorization: Bearer {token}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    s.write_all(req.as_bytes()).expect("write");
    s.flush().expect("flush");

    let mut reader = BufReader::new(s);
    let mut status = String::new();
    reader.read_line(&mut status).expect("status");
    let mut chunked = false;
    let mut content_length: Option<usize> = None;
    loop {
        let mut h = String::new();
        reader.read_line(&mut h).expect("header");
        if h.trim().is_empty() {
            break;
        }
        let lower = h.to_ascii_lowercase();
        if lower.contains("transfer-encoding: chunked") {
            chunked = true;
        }
        if let Some(v) = lower.strip_prefix("content-length:") {
            content_length = v.trim().parse().ok();
        }
    }
    if chunked {
        let mut out = Vec::new();
        loop {
            let mut size_line = String::new();
            reader.read_line(&mut size_line).expect("chunk size");
            let size = usize::from_str_radix(size_line.trim(), 16).unwrap_or(0);
            if size == 0 {
                let mut end = String::new();
                let _ = reader.read_line(&mut end);
                break;
            }
            let mut chunk = vec![0u8; size];
            reader.read_exact(&mut chunk).expect("chunk");
            out.extend_from_slice(&chunk);
            let mut crlf = String::new();
            let _ = reader.read_line(&mut crlf);
        }
        return (status, String::from_utf8_lossy(&out).to_string());
    }
    let mut out = vec![0u8; content_length.unwrap_or(0)];
    if !out.is_empty() {
        reader.read_exact(&mut out).expect("body");
    }
    (status, String::from_utf8_lossy(&out).to_string())
}

const SSE: &str = "event: message_start\ndata: {\"type\":\"message_start\"}\n\nevent: message_stop\ndata: {\"type\":\"message_stop\"}\n\n";

#[test]
fn refuses_a_request_without_the_proxy_token_before_contacting_any_upstream() {
    let up = spawn_upstream(vec![(200, SSE.to_string())]);
    let (_home, handle) = setup("auth", vec![target("t1", &up.addr, None)]);
    let (status, _body) = call(handle.port, "wrong-token", "{\"model\":\"m\"}");
    assert!(status.contains("401"), "{status}");
    assert_eq!(
        up.hits.load(Ordering::SeqCst),
        0,
        "upstream must not be called"
    );
}

#[test]
fn streams_the_upstream_body_through() {
    let up = spawn_upstream(vec![(200, SSE.to_string())]);
    let (_home, handle) = setup("stream", vec![target("t1", &up.addr, None)]);
    let (status, body) = call(
        handle.port,
        &handle.token,
        "{\"model\":\"m\",\"stream\":true}",
    );
    assert!(status.contains("200"), "{status}");
    assert_eq!(body, SSE);
    assert_eq!(up.hits.load(Ordering::SeqCst), 1);
    let st = handle.status();
    assert_eq!(st.last_target.as_deref(), Some("t1"));
    assert_eq!(st.fallbacks, 0);
    assert_eq!(st.requests, 1);
}

#[test]
fn falls_back_on_rate_limit_and_reports_it() {
    let first = spawn_upstream(vec![(429, "{\"error\":\"slow down\"}".to_string())]);
    let second = spawn_upstream(vec![(200, "{\"ok\":true}".to_string())]);
    let (_home, handle) = setup(
        "fallback",
        vec![
            target("t1", &first.addr, None),
            target("t2", &second.addr, None),
        ],
    );
    let (status, body) = call(handle.port, &handle.token, "{\"model\":\"m\"}");
    assert!(status.contains("200"), "{status}");
    assert_eq!(body, "{\"ok\":true}");
    assert_eq!(first.hits.load(Ordering::SeqCst), 1);
    assert_eq!(second.hits.load(Ordering::SeqCst), 1);
    let st = handle.status();
    assert_eq!(st.last_target.as_deref(), Some("t2"));
    assert_eq!(st.fallbacks, 1);
    assert_eq!(st.cooling, vec!["t1".to_string()]);
}

#[test]
fn does_not_fail_over_on_a_request_error() {
    let first = spawn_upstream(vec![(400, "{\"error\":\"bad body\"}".to_string())]);
    let second = spawn_upstream(vec![(200, "{\"ok\":true}".to_string())]);
    let (_home, handle) = setup(
        "request-error",
        vec![
            target("t1", &first.addr, None),
            target("t2", &second.addr, None),
        ],
    );
    let (status, body) = call(handle.port, &handle.token, "{\"model\":\"m\"}");
    assert!(
        status.contains("400"),
        "the caller's own error must pass through: {status}"
    );
    assert!(body.contains("bad body"));
    assert_eq!(
        second.hits.load(Ordering::SeqCst),
        0,
        "no fallback for a bad request"
    );
}

#[test]
fn replaces_the_model_when_a_target_pins_one() {
    let up = spawn_upstream(vec![(200, "{}".to_string())]);
    let (_home, handle) = setup(
        "rewrite",
        vec![target("t1", &up.addr, Some("nvidia/nemotron-3-ultra"))],
    );
    let (status, _) = call(
        handle.port,
        &handle.token,
        "{\"model\":\"caller/model\",\"max_tokens\":8}",
    );
    assert!(status.contains("200"), "{status}");
    let bodies = up.bodies.lock().expect("bodies");
    let sent: serde_json::Value = serde_json::from_str(&bodies[0]).expect("json");
    assert_eq!(sent["model"], "nvidia/nemotron-3-ultra");
    assert_eq!(sent["max_tokens"], 8);
}

#[test]
fn reports_all_targets_failed_without_leaking_key_material() {
    let a = spawn_upstream(vec![(503, "busy".to_string())]);
    let b = spawn_upstream(vec![(503, "busy".to_string())]);
    let (_home, handle) = setup(
        "all-fail",
        vec![target("t1", &a.addr, None), target("t2", &b.addr, None)],
    );
    let (status, body) = call(handle.port, &handle.token, "{\"model\":\"m\"}");
    assert!(status.contains("503"), "{status}");
    assert!(body.contains("all targets failed"));
    assert!(!body.contains("sk-or-v1-fixture-key"));
    assert_eq!(handle.status().failures, 1);
}

/// The whole chain the owner gets: store a key, save a routing target, point
/// Claude Code at the proxy, then use the generated helper's token to call it.
#[test]
fn routed_mode_end_to_end_through_the_generated_helper() {
    let up = spawn_upstream(vec![(200, "{\"answered\":true}".to_string())]);
    let home = temp_home("e2e");
    std::fs::create_dir_all(home.join(".claude")).expect("claude dir");
    std::fs::write(home.join(".claude/settings.json"), "{\"model\":\"old\"}").expect("seed");

    let vault = CredentialVault::for_home(&home);
    vault
        .add("openrouter", "primary", "sk-or-v1-e2e-fixture")
        .expect("credential");
    routing_config::save(
        &home,
        &RoutingConfig {
            version: routing_config::ROUTING_VERSION,
            enabled: true,
            targets: vec![target("t1", &up.addr, Some("pinned/model"))],
        },
    )
    .expect("routing");

    let handle = proxy::start(home.clone()).expect("proxy");
    let svc = claude_config_manager_lib::save_apply::SaveApplyService::for_production(
        &home,
        std::sync::Arc::new(
            claude_config_manager_lib::file_secret_store::FileSecretStore::for_home(&home),
        ),
    );
    let outcome = svc
        .apply_routed(handle.port, &handle.token, Some("pinned/model"))
        .expect("apply");

    // The helper prints the proxy token, and that token authenticates the call.
    let printed = std::process::Command::new("sh")
        .arg(&outcome.helper_path)
        .env("HOME", &home)
        .output()
        .expect("helper");
    assert!(printed.status.success(), "helper failed: {printed:?}");
    let token = String::from_utf8_lossy(&printed.stdout).trim().to_string();
    assert_eq!(token, handle.token);

    let (status, body) = call(handle.port, &token, "{\"model\":\"caller/model\"}");
    assert!(status.contains("200"), "{status}");
    assert!(body.contains("answered"));

    // The upstream saw the target's model, and only the provider credential.
    let bodies = up.bodies.lock().expect("bodies");
    assert!(bodies[0].contains("pinned/model"));
    assert!(!bodies[0].contains("sk-or-v1-e2e-fixture"));

    // Claude Code's own settings never hold a key, only the loopback endpoint.
    let settings = std::fs::read_to_string(home.join(".claude/settings.json")).expect("settings");
    assert!(settings.contains(&format!("http://127.0.0.1:{}", handle.port)));
    assert!(!settings.contains("sk-or-v1"));
    assert!(!settings.contains(&handle.token));
}

#[test]
fn shutdown_stops_accepting_requests() {
    let up = spawn_upstream(vec![(200, "{}".to_string())]);
    let (_home, mut handle) = setup("shutdown", vec![target("t1", &up.addr, None)]);
    let port = handle.port;
    handle.shutdown();
    assert!(TcpStream::connect(("127.0.0.1", port)).is_err() || !handle.status().running);
}
