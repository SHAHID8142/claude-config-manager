//! V2-29/31/35/36: the loopback failover proxy.
//!
//! Claude Code talks to `http://127.0.0.1:<port>` and authenticates with a
//! per-process bearer token. The proxy resolves the real credential from the
//! owner-only store, tries the configured targets in order, and streams the
//! winning answer back verbatim (SSE included).
//!
//! Safety properties, in the order they matter:
//! * **Bound to loopback only**, ephemeral port, per-process random token. A
//!   request without the token is refused before any upstream call.
//! * **Never replays a started stream** — the client only receives bytes after
//!   an upstream target already answered successfully.
//! * **Never retries a request error** (400/404/422): another provider would
//!   reject the same body, and retrying could duplicate a side effect.
//! * **No key material in logs or status** — only target ids and failure classes.
//! * **Bounded** bodies (16 MiB), headers (64 KiB) and timeouts (120 s), so a
//!   stuck request cannot wedge the app.

use crate::failover::{self, Cooldowns, FailureClass};
use crate::provider_catalog::AuthStyle;
use crate::routing_config::{self, RouteTarget, RoutingConfig};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const MAX_HEADER_BYTES: usize = 64 * 1024;
const MAX_BODY_BYTES: usize = 16 * 1024 * 1024;
const CLIENT_TIMEOUT: Duration = Duration::from_secs(120);
const UPSTREAM_TIMEOUT: Duration = Duration::from_secs(120);

fn err_body(kind: &str, message: &str) -> String {
    serde_json::json!({
        "type": "error",
        "error": { "type": kind, "message": message }
    })
    .to_string()
}

/// Live status for the UI. Contains no credentials and no response bodies.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyStatus {
    pub running: bool,
    pub port: Option<u16>,
    pub requests: u64,
    pub fallbacks: u64,
    pub failures: u64,
    pub last_target: Option<String>,
    pub last_error: Option<String>,
    /// Target ids currently in cooldown (never key material).
    pub cooling: Vec<String>,
    /// Owning process. The GUI uses it to prove that the pid in its state file
    /// is the process actually answering on this port before signalling it.
    pub pid: u32,
}

pub struct ProxyHandle {
    pub port: u16,
    /// Bearer token Claude Code must send. Also written into the helper script.
    pub token: String,
    shutdown: Arc<AtomicBool>,
    status: Arc<Mutex<ProxyStatus>>,
    cooldowns: Arc<Mutex<Cooldowns>>,
    home: PathBuf,
    accept: Option<std::thread::JoinHandle<()>>,
}

impl ProxyHandle {
    pub fn status(&self) -> ProxyStatus {
        let mut s = self
            .status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        s.running = !self.shutdown.load(Ordering::SeqCst);
        s.port = Some(self.port);
        s.cooling = self
            .cooldowns
            .lock()
            .map(|c| {
                // Reconstruct visible cooldowns by probing known target ids.
                let mut out = Vec::new();
                if let Ok(cfg) = routing_config::load(&self.home) {
                    let now = Instant::now();
                    let mut guard = c;
                    for t in &cfg.targets {
                        if guard.is_cooling(&t.id, now) {
                            out.push(t.id.clone());
                        }
                    }
                }
                out
            })
            .unwrap_or_default();
        s
    }

    pub fn shutdown(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        // Unblock accept() with a throwaway connection.
        let _ = TcpStream::connect(("127.0.0.1", self.port)).map(|s| s.shutdown(Shutdown::Both));
        if let Some(h) = self.accept.take() {
            let _ = h.join();
        }
    }
}

impl Drop for ProxyHandle {
    fn drop(&mut self) {
        if !self.shutdown.load(Ordering::SeqCst) {
            self.shutdown();
        }
    }
}

pub fn random_token() -> String {
    // V3-06: 256 bits from the operating system CSPRNG, hex-encoded. The token
    // gates the loopback proxy, so a predictable value would let any local
    // process spend the owner's provider quota.
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("the OS random source must be available");
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Start the proxy on an ephemeral loopback port with a fresh token.
pub fn start(home: PathBuf) -> Result<ProxyHandle, String> {
    start_with(home, None, None)
}

/// Start the proxy on a preferred loopback port with a caller-supplied token.
///
/// The helper uses this so a restart keeps both the port Claude Code is already
/// pointed at and the bearer token its helper script already contains. If the
/// preferred port is taken the OS picks another one and the caller must repoint
/// Claude Code (that is what `apply_routed_mode` does).
pub fn start_with(
    home: PathBuf,
    preferred_port: Option<u16>,
    token: Option<String>,
) -> Result<ProxyHandle, String> {
    let bind = |port: u16| TcpListener::bind(("127.0.0.1", port));
    let listener = match preferred_port {
        Some(p) => match bind(p) {
            Ok(l) => l,
            Err(_) => bind(0).map_err(|e| format!("bind: {e}"))?,
        },
        None => bind(0).map_err(|e| format!("bind: {e}"))?,
    };
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let token = token.unwrap_or_else(random_token);
    let shutdown = Arc::new(AtomicBool::new(false));
    let status = Arc::new(Mutex::new(ProxyStatus {
        running: true,
        port: Some(port),
        pid: std::process::id(),
        ..Default::default()
    }));
    let cooldowns = Arc::new(Mutex::new(Cooldowns::new()));

    let ctx = Arc::new(Ctx {
        home: home.clone(),
        token: token.clone(),
        status: Arc::clone(&status),
        cooldowns: Arc::clone(&cooldowns),
        shutdown: Arc::clone(&shutdown),
    });

    let accept_shutdown = Arc::clone(&shutdown);
    let accept = std::thread::Builder::new()
        .name("ccm-proxy".into())
        .spawn(move || {
            for conn in listener.incoming() {
                if accept_shutdown.load(Ordering::SeqCst) {
                    break;
                }
                match conn {
                    Ok(stream) => {
                        let ctx = Arc::clone(&ctx);
                        // One thread per request: Claude Code issues few, long
                        // requests; a pool would add failure modes for nothing.
                        let _ = std::thread::Builder::new()
                            .name("ccm-proxy-req".into())
                            .spawn(move || {
                                let _ = handle_conn(stream, &ctx);
                            });
                    }
                    Err(_) => break,
                }
            }
        })
        .map_err(|e| format!("spawn: {e}"))?;

    Ok(ProxyHandle {
        port,
        token,
        shutdown,
        status,
        cooldowns,
        home,
        accept: Some(accept),
    })
}

struct Ctx {
    home: PathBuf,
    token: String,
    status: Arc<Mutex<ProxyStatus>>,
    cooldowns: Arc<Mutex<Cooldowns>>,
    shutdown: Arc<AtomicBool>,
}

/// CORS-free, loopback-only, token-gated. Returns after the response is written.
fn handle_conn(mut stream: TcpStream, ctx: &Ctx) -> std::io::Result<()> {
    stream.set_read_timeout(Some(CLIENT_TIMEOUT))?;
    stream.set_write_timeout(Some(CLIENT_TIMEOUT))?;
    let mut reader = BufReader::new(stream.try_clone()?);

    let (method, path, headers) = match read_head(&mut reader) {
        Ok(v) => v,
        Err(resp) => return write_simple(&mut stream, resp.0, "text/plain", &resp.1),
    };

    if method == "GET" && (path == "/healthz" || path == "/status") {
        let body = serde_json::to_string(&ctx.snapshot()).unwrap_or_else(|_| "{}".into());
        return write_simple(&mut stream, 200, "application/json", &body);
    }

    // Exact path only: the upstream URL is `<base>/v1/messages`, so anything
    // else (including `/v1/messages/../x`) is a caller error, not a proxy job.
    let route = path.split('?').next().unwrap_or("");
    if method != "POST" || route != "/v1/messages" {
        return write_simple(
            &mut stream,
            404,
            "application/json",
            &err_body("not_found", "only POST /v1/messages is proxied"),
        );
    }

    match authorize(&headers, &ctx.token) {
        Ok(()) => {}
        Err(msg) => {
            bump(&ctx.status, |s| s.failures += 1);
            return write_simple(
                &mut stream,
                401,
                "application/json",
                &err_body("authentication_error", &msg),
            );
        }
    }

    let body = match read_body(&mut reader, &headers) {
        Ok(b) => b,
        Err(e) => {
            bump(&ctx.status, |s| {
                s.failures += 1;
                s.last_error = Some(e.clone());
            });
            return write_simple(
                &mut stream,
                413,
                "application/json",
                &err_body("invalid_request_error", &e),
            );
        }
    };

    let cfg = match routing_config::load(&ctx.home) {
        Ok(c) => c,
        Err(e) => {
            return write_simple(
                &mut stream,
                500,
                "application/json",
                &err_body(
                    "proxy_unavailable",
                    &format!("routing config unreadable: {e}"),
                ),
            );
        }
    };

    let attempt = forward(&body, &headers, &cfg, ctx);
    match attempt {
        Ok(Final::Rejected { status, body }) => {
            bump(&ctx.status, |s| {
                s.requests += 1;
                s.failures += 1;
                s.last_error = Some(format!("request rejected by upstream ({status})"));
            });
            write_simple(&mut stream, status, "application/json", &body)
        }
        Ok(Final::Stream(mut ok)) => {
            bump(&ctx.status, |s| {
                s.requests += 1;
                if ok.attempts > 1 {
                    s.fallbacks += 1;
                }
                s.last_target = Some(ok.target_id.clone());
                s.last_error = None;
            });
            stream_response(&mut stream, &mut ok.response)
        }
        Err(e) => {
            bump(&ctx.status, |s| {
                s.requests += 1;
                s.failures += 1;
                s.last_error = Some(e.message.clone());
            });
            write_simple(
                &mut stream,
                e.status,
                "application/json",
                &err_body(e.kind, &e.message),
            )
        }
    }
}

impl Ctx {
    fn snapshot(&self) -> ProxyStatus {
        let mut s = self.status.lock().map(|s| s.clone()).unwrap_or_default();
        s.running = !self.shutdown.load(Ordering::SeqCst);
        s.pid = std::process::id();
        let now = Instant::now();
        if let (Ok(mut c), Ok(cfg)) = (self.cooldowns.lock(), routing_config::load(&self.home)) {
            s.cooling = cfg
                .targets
                .iter()
                .filter(|t| c.is_cooling(&t.id, now))
                .map(|t| t.id.clone())
                .collect();
        }
        s
    }
}

fn bump(status: &Arc<Mutex<ProxyStatus>>, f: impl FnOnce(&mut ProxyStatus)) {
    if let Ok(mut s) = status.lock() {
        f(&mut s);
    }
}

type HttpHead = (String, String, HashMap<String, String>);

/// Parses the request line and headers. Errors are pre-built responses.
fn read_head(r: &mut BufReader<TcpStream>) -> Result<HttpHead, (u16, String)> {
    let mut line = String::new();
    if r.by_ref()
        .take(MAX_HEADER_BYTES as u64)
        .read_line(&mut line)
        .is_err()
    {
        return Err((400, "malformed request".into()));
    }
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or_default().to_string();
    if method.is_empty() || path.is_empty() {
        return Err((400, "malformed request line".into()));
    }

    let mut headers = HashMap::new();
    let mut total = line.len();
    loop {
        let mut h = String::new();
        if r.read_line(&mut h).is_err() {
            return Err((400, "malformed headers".into()));
        }
        total += h.len();
        if total > MAX_HEADER_BYTES {
            return Err((431, "request headers too large".into()));
        }
        let trimmed = h.trim_end();
        if trimmed.is_empty() {
            break;
        }
        if let Some((k, v)) = trimmed.split_once(':') {
            headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }
    Ok((method, path, headers))
}

fn authorize(headers: &HashMap<String, String>, token: &str) -> Result<(), String> {
    let given = headers
        .get("x-api-key")
        .cloned()
        .or_else(|| {
            headers
                .get("authorization")
                .and_then(|v| v.strip_prefix("Bearer ").map(|s| s.to_string()))
        })
        .unwrap_or_default();
    if given.is_empty() {
        return Err("missing proxy token".into());
    }
    // Compare without short-circuiting on the first differing byte, so the
    // token check does not leak a prefix match through timing.
    if ct_eq(given.as_bytes(), token.as_bytes()) {
        Ok(())
    } else {
        Err("invalid proxy token".into())
    }
}

fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn read_body(
    r: &mut BufReader<TcpStream>,
    headers: &HashMap<String, String>,
) -> Result<Vec<u8>, String> {
    let chunked = headers
        .get("transfer-encoding")
        .map(|v| v.to_ascii_lowercase().contains("chunked"))
        .unwrap_or(false);
    if chunked {
        return read_chunked(r);
    }
    let len: usize = headers
        .get("content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    if len > MAX_BODY_BYTES {
        return Err("request body too large".into());
    }
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf).map_err(|e| e.to_string())?;
    Ok(buf)
}

fn read_chunked(r: &mut BufReader<TcpStream>) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    loop {
        let mut size_line = String::new();
        r.read_line(&mut size_line).map_err(|e| e.to_string())?;
        let size = usize::from_str_radix(size_line.trim().split(';').next().unwrap_or("0"), 16)
            .map_err(|_| "bad chunk size".to_string())?;
        if size == 0 {
            let mut trailer = String::new();
            // Consume the final CRLF / trailers.
            let _ = r.read_line(&mut trailer);
            return Ok(out);
        }
        if out.len() + size > MAX_BODY_BYTES {
            return Err("request body too large".into());
        }
        let mut chunk = vec![0u8; size];
        r.read_exact(&mut chunk).map_err(|e| e.to_string())?;
        out.extend_from_slice(&chunk);
        let mut crlf = String::new();
        r.read_line(&mut crlf).map_err(|e| e.to_string())?;
    }
}

fn write_simple(
    stream: &mut TcpStream,
    status: u16,
    ctype: &str,
    body: &str,
) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 {status} {}\r\ncontent-type: {ctype}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
        reason(status),
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body.as_bytes())?;
    stream.flush()
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        413 => "Payload Too Large",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        _ => "Error",
    }
}

struct Upstream {
    target_id: String,
    response: ureq::http::Response<ureq::Body>,
    attempts: usize,
}

/// What the proxy finally sends back to Claude Code.
enum Final {
    /// A successful upstream answer, streamed verbatim.
    Stream(Box<Upstream>),
    /// The caller's own request was rejected (400/404/422): pass the status and
    /// the provider's explanation through, never a synthetic error.
    Rejected { status: u16, body: String },
}

/// V3-13/V3-16: which wire protocol a provider speaks.
///
/// The proxy forwards the Anthropic Messages API verbatim. A provider that only
/// speaks OpenAI Chat Completions cannot be served without a translation layer,
/// so the proxy refuses it explicitly instead of sending a body the provider
/// will reject with an opaque 400. Capabilities are never silently dropped: an
/// unbridged provider produces a compatibility error naming the reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Adapter {
    AnthropicNative,
    /// Speaks the Anthropic API (native or compatible), forwarded verbatim.
    AnthropicCompatible,
    /// OpenAI Chat Completions only; needs a bridge this build does not ship.
    OpenAiUnbridged,
}

pub fn adapter_for(provider_id: &str) -> Option<Adapter> {
    use crate::provider_catalog::Protocol;
    let templates = crate::presets::load_builtin_presets();
    let t = templates.iter().find(|t| t.id == provider_id)?;
    Some(match t.protocol {
        Protocol::AnthropicNative => Adapter::AnthropicNative,
        Protocol::AnthropicCompat => Adapter::AnthropicCompatible,
        Protocol::OpenAiOnly => Adapter::OpenAiUnbridged,
    })
}

/// One sentence naming the missing piece. Never includes key material.
pub fn bridge_requirement_message(provider_id: &str) -> String {
    format!(
        "{provider_id} speaks the OpenAI Chat Completions API. This proxy forwards the Anthropic \
         Messages API and ships no protocol bridge, so the request was not sent. Choose an \
         Anthropic-compatible provider, or install a bridge and point the target at it."
    )
}

/// A failure that is *ours*, with the status and error type Claude Code sees.
struct ProxyFailure {
    status: u16,
    kind: &'static str,
    message: String,
}

impl ProxyFailure {
    fn unavailable(message: String) -> Self {
        Self {
            status: 503,
            kind: "proxy_unavailable",
            message,
        }
    }
    fn incompatible(message: String) -> Self {
        Self {
            status: 501,
            kind: "compatibility_error",
            message,
        }
    }
}

/// Try targets in order. Returns the first successful upstream response, or a
/// redacted description of why every target failed.
fn forward(
    body: &[u8],
    client_headers: &HashMap<String, String>,
    cfg: &RoutingConfig,
    ctx: &Ctx,
) -> Result<Final, ProxyFailure> {
    let ordered = cfg.ordered();
    if ordered.is_empty() {
        return Err(ProxyFailure::unavailable(
            "no routing targets are enabled — add one in the app".into(),
        ));
    }
    let ids: Vec<(String, bool)> = ordered.iter().map(|t| (t.id.clone(), true)).collect();
    let now = Instant::now();
    let candidates = {
        let mut cd = ctx
            .cooldowns
            .lock()
            .map_err(|_| ProxyFailure::unavailable("proxy state poisoned".into()))?;
        failover::candidates(&ids, &mut cd, now)
    };

    // Refuse a chain that can only be served through a bridge, before any
    // network call, so the caller gets one clear answer instead of a 400.
    let any_usable = ordered
        .iter()
        .any(|t| adapter_for(&t.provider_id) != Some(Adapter::OpenAiUnbridged));
    if !any_usable {
        let first = ordered
            .first()
            .map(|t| t.provider_id.clone())
            .unwrap_or_default();
        return Err(ProxyFailure::incompatible(bridge_requirement_message(
            &first,
        )));
    }

    let mut attempts = 0usize;
    let mut reasons: Vec<String> = Vec::new();
    for id in candidates {
        let Some(target) = ordered.iter().find(|t| t.id == id).copied() else {
            continue;
        };
        if adapter_for(&target.provider_id) == Some(Adapter::OpenAiUnbridged) {
            reasons.push(format!("{}: needs a protocol bridge", target.provider_id));
            continue;
        }
        attempts += 1;
        match try_target(target, body, client_headers, ctx) {
            Ok(TargetOutcome::Answered(resp)) => {
                if let Ok(mut cd) = ctx.cooldowns.lock() {
                    cd.clear(&id);
                }
                return Ok(Final::Stream(Box::new(Upstream {
                    target_id: id,
                    response: resp,
                    attempts,
                })));
            }
            Ok(TargetOutcome::Rejected { status, body }) => {
                return Ok(Final::Rejected { status, body });
            }
            Err(AttemptError { class, detail }) => {
                if let Ok(mut cd) = ctx.cooldowns.lock() {
                    cd.mark(&id, class, detail.retry_after, now);
                }
                reasons.push(format!("{}: {}", id, class.as_str()));
                if !class.retryable() {
                    break;
                }
            }
        }
    }
    if reasons.is_empty() {
        return Err(ProxyFailure::unavailable(
            "no routing target could be used".into(),
        ));
    }
    Err(ProxyFailure::unavailable(format!(
        "all targets failed — {}",
        reasons.join(", ")
    )))
}

struct AttemptError {
    class: FailureClass,
    detail: RetryDetail,
}

struct RetryDetail {
    retry_after: Option<Duration>,
}

impl RetryDetail {
    fn none() -> Self {
        Self { retry_after: None }
    }
}

enum TargetOutcome {
    Answered(ureq::http::Response<ureq::Body>),
    Rejected { status: u16, body: String },
}

fn try_target(
    target: &RouteTarget,
    body: &[u8],
    client_headers: &HashMap<String, String>,
    ctx: &Ctx,
) -> Result<TargetOutcome, AttemptError> {
    let secret = resolve_secret(&ctx.home, target).ok_or(AttemptError {
        class: FailureClass::Auth,
        detail: RetryDetail::none(),
    })?;

    let url = format!("{}/v1/messages", target.base_url.trim_end_matches('/'));
    let payload = rewrite_model(body, target.model.as_deref()).map_err(|_| AttemptError {
        class: FailureClass::Request,
        detail: RetryDetail::none(),
    })?;

    let auth = provider_auth_style(&target.provider_id);
    let mut req = ureq::post(&url).header("content-type", "application/json");
    req = match auth {
        AuthStyle::ApiKey => req.header("x-api-key", &secret),
        AuthStyle::BearerToken => req.header("authorization", &format!("Bearer {secret}")),
    };
    // Pass through only protocol headers Claude Code owns.
    for name in ["anthropic-version", "anthropic-beta", "accept"] {
        if let Some(v) = client_headers.get(name) {
            req = req.header(name, v);
        }
    }
    match req
        .config()
        .timeout_global(Some(UPSTREAM_TIMEOUT))
        // Statuses are data here: the proxy decides whether a status is a
        // retryable provider failure or the caller's own error.
        .http_status_as_error(false)
        .max_redirects(0)
        .build()
        .send(payload.as_slice())
    {
        Ok(mut resp) => {
            let status = resp.status().as_u16();
            if (200..300).contains(&status) {
                return Ok(TargetOutcome::Answered(resp));
            }
            let retry_after = failover::parse_retry_after(
                resp.headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok()),
            );
            // Bounded prefix: enough to tell "rate limited" from "out of
            // credit", never logged.
            let mut prefix = String::new();
            let _ = resp
                .body_mut()
                .as_reader()
                .take(8192)
                .read_to_string(&mut prefix);
            let class = failover::classify(status, &prefix);
            if class == FailureClass::Request {
                return Ok(TargetOutcome::Rejected {
                    status,
                    body: prefix,
                });
            }
            Err(AttemptError {
                class,
                detail: RetryDetail { retry_after },
            })
        }
        Err(ureq::Error::StatusCode(code)) => Err(AttemptError {
            class: failover::classify(code, ""),
            detail: RetryDetail::none(),
        }),
        Err(ureq::Error::Timeout(_)) => Err(AttemptError {
            class: FailureClass::Timeout,
            detail: RetryDetail::none(),
        }),
        Err(_) => Err(AttemptError {
            class: FailureClass::Network,
            detail: RetryDetail::none(),
        }),
    }
}

fn rewrite_model(body: &[u8], model: Option<&str>) -> Result<Vec<u8>, String> {
    let Some(model) = model else {
        return Ok(body.to_vec());
    };
    let mut v: serde_json::Value =
        serde_json::from_slice(body).map_err(|e| format!("request body is not JSON: {e}"))?;
    let Some(obj) = v.as_object_mut() else {
        return Err("request body must be a JSON object".into());
    };
    obj.insert("model".into(), serde_json::Value::String(model.into()));
    serde_json::to_vec(&v).map_err(|e| e.to_string())
}

fn provider_auth_style(provider_id: &str) -> AuthStyle {
    crate::presets::load_builtin_presets()
        .iter()
        .find(|t| t.id == provider_id)
        .map(|t| t.auth)
        .unwrap_or(AuthStyle::BearerToken)
}

/// Credential for a target: the named credential, else the provider's own file.
pub fn resolve_secret(home: &std::path::Path, target: &RouteTarget) -> Option<String> {
    let vault = crate::credential_vault::CredentialVault::for_home(home);
    if let Some(id) = &target.credential_id {
        // V3-20: a credential may only serve the provider it was stored for.
        // Without this check a hand-edited routing.json could send provider B's
        // key to provider A's endpoint.
        let record = vault
            .list(&target.provider_id)
            .ok()
            .and_then(|list| list.into_iter().find(|c| &c.id == id));
        match record {
            Some(rec) if rec.provider_id == target.provider_id && rec.enabled => {
                if let Ok(Some(s)) = vault.secret(id) {
                    return Some(s);
                }
            }
            // Another provider's key, or a disabled one: never used.
            Some(_) => return None,
            None => {
                // The index may not know it yet (fresh file, migrated install).
                // Fall through to the provider's own legacy file only when the
                // target's credential id *is* the provider id.
                if id != &target.provider_id {
                    return None;
                }
            }
        }
    }
    if let Ok(Some(s)) = vault.secret(&target.provider_id) {
        return Some(s);
    }
    // Legacy file that the index does not know about yet.
    let store = crate::file_secret_store::FileSecretStore::for_home(home);
    crate::secret_store::SecretStore::get_secret(
        &store,
        &format!("com.claude-config-manager.{}", target.provider_id),
        &target.provider_id,
    )
    .ok()
    .flatten()
}

/// Stream the upstream body to the client with chunked encoding, flushing per
/// chunk so SSE deltas reach Claude Code immediately.
fn stream_response(
    stream: &mut TcpStream,
    resp: &mut ureq::http::Response<ureq::Body>,
) -> std::io::Result<()> {
    let status = resp.status().as_u16();
    let ctype = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json")
        .to_string();

    let head = format!(
        "HTTP/1.1 {status} {}\r\ncontent-type: {ctype}\r\ntransfer-encoding: chunked\r\nconnection: close\r\n\r\n",
        reason(status)
    );
    stream.write_all(head.as_bytes())?;

    let mut reader = resp.body_mut().as_reader();
    let mut buf = [0u8; 8192];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                stream.write_all(format!("{n:x}\r\n").as_bytes())?;
                stream.write_all(&buf[..n])?;
                stream.write_all(b"\r\n")?;
                stream.flush()?;
            }
            Err(e) => {
                // Abort the chunked stream so the client sees a truncated body
                // rather than a clean end (which would look like a full answer).
                return Err(e);
            }
        }
    }
    stream.write_all(b"0\r\n\r\n")?;
    stream.flush()
}
