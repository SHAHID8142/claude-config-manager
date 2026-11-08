# V2-23 … V2-31, V2-33 … V2-41 — routing chain, loopback proxy, failover policy

Evidence for the failover milestone. Everything here is code that ships in this repo — no
third-party router binary is installed, downloaded or executed (ADR-003).

## What exists

| Task | Where | Notes |
| --- | --- | --- |
| V2-23 Multiple routing targets | `src-tauri/src/routing_config.rs` | `RoutingConfig { version, enabled, targets[] }`, ≤ 10 targets, unique valid ids, https-only except loopback |
| V2-24 Compact priority editor | `src/components/RoutingPanel.tsx` | ordered list, ↑/↓, remove, add row, `role=switch`-style toggle via labelled checkbox |
| V2-25 Migration/permission/redaction/isolation | same modules | index version gate, 0600 writes, atomic rename, secret refs stripped from errors, proxy bound to loopback |
| V2-26 Router contract | `src-tauri/src/proxy.rs` | own Rust implementation; no external release to verify (see ADR-003) |
| V2-27 Third-party consent / pinned download | — | **not applicable**: nothing is downloaded. This is the reason ADR-003 chose an in-repo proxy. |
| V2-28 Router config generation | `routing_config::save` | JSON at `~/.claude-config-manager/routing.json`, 0600, metadata only (credential **ids**, never keys) |
| V2-29 Loopback routing + startup | `proxy::start` | bind `127.0.0.1:0`, ephemeral port, per-process random token |
| V2-30 Lifecycle / recovery | `ProxyHandle::{status,shutdown}` + `Drop` | stop closes the listener; the app holds the only handle; `proxy_status` reports counters |
| V2-31 Endpoint into Claude Code config | `SaveApplyService::apply_routed` | `ANTHROPIC_BASE_URL=http://127.0.0.1:<port>`, `apiKeyHelper=…/helpers/proxy.sh`, plaintext env creds cleared |
| V2-33 Error classification | `failover::classify` | 401/403 auth, 429 rate-limit vs quota body, 5xx, timeouts, network, request errors |
| V2-34 Rate limit vs quota | `failover::{QUOTA_HINTS, classify}` | quota is only inferred from an explicit body phrase; a bare 429 is never treated as permanent |
| V2-35 Bounded retry + cooldown | `failover::{cooldown_for, Cooldowns, parse_retry_after}` | quota/auth 6 h, rate-limit honours `Retry-After` (cap 600 s) else 30 s, server 60 s |
| V2-36 Ordered fallback | `failover::candidates` + `proxy::forward` | top-to-bottom, cooling targets skipped unless every target is cooling |
| V2-37 Cross-provider fallback | same | targets name their own provider, endpoint, credential and optional model |
| V2-38 Auto-failover toggle | `RoutingConfig.enabled` + RoutingPanel | **off by default**; `apply_routed_mode` refuses while off |
| V2-39 Status + safe diagnostics | `ProxyStatus` | counters, last target id, cooling ids, redacted error string. No bodies, no keys |
| V2-40 Exhausted/invalid/timeout/5xx tests | `tests/proxy_loopback.rs`, `failover` unit tests | see below |
| V2-41 Streaming, tools, replay safety | `proxy::stream_response` | upstream bytes forwarded chunk-by-chunk, flushed per chunk; a started stream is never replayed |

## Behaviour that is deliberately conservative

* **Loopback only.** The listener is `127.0.0.1` with an ephemeral port and a 128-bit token.
  A request without the token gets `401` before any upstream connection is opened
  (`refuses_a_request_without_the_proxy_token_before_contacting_any_upstream`).
* **No fallback on the caller's own error.** 400/404/422 are handed back unchanged, so a bad
  request is never replayed at a second provider and nothing is charged twice
  (`does_not_fail_over_on_a_request_error`).
* **A bare 429 is rate limiting, not exhaustion.** Cooldown is 30 s (or `Retry-After`), so a
  provider's temporary limit does not silently demote it for hours
  (`failover::tests::quota_is_distinguished_from_a_plain_rate_limit`).
* **A started stream is never retried.** Failover happens only before the first byte is
  written to Claude Code, which is why the proxy buffers no more than the request body.
* **Keys never leave the owner's store.** Only the proxy reads `secrets/<credential-id>.secret`;
  settings hold the loopback URL plus a helper that prints the *proxy* token
  (`routed_mode_end_to_end_through_the_generated_helper` asserts `sk-or-v1` never appears in
  `settings.json`).
* **Cooldowns never empty the chain.** If every target is cooling down, the order is reused
  rather than failing with "no targets" (`failover::tests::all_cooling_still_returns_targets`).

## Verification (code-level, per owner instruction)

```
cargo test         291 passed / 0 failed   (8 proxy-loopback tests, 7 routing, 5 failover)
cargo clippy       0 warnings
npx tsc --noEmit   clean
npm run lint       clean
npx vitest run     177 passed / 22 files
npm run tauri build exit 0
```

## Known limitations

* Keyboard shortcuts and health checks are limited to what the panel exposes: start/stop,
  apply, and per-target reordering. There is no scheduled background probe (V2-32 partially
  covered by on-demand `proxy_status` and the existing **Test connection** action).
* Cooldowns live in memory for the process lifetime; restarting the app clears them.
* The proxy forwards `anthropic-version`, `anthropic-beta` and `accept` only. A provider that
  needs another protocol header cannot be routed yet.
* Failover is not exercised against a live paid provider in CI — the tests use a scripted
  loopback upstream so no real key or charge is involved.
