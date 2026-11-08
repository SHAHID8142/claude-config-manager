# T37 — Provider model discovery interface + mock API

**Date:** 2026-11-08 · **Result:** ✅ PASS (3 focused fix cycles for the ureq v3 API — within budget) · **Artifact:** `src-tauri/src/model_discovery.rs` (+ `ureq` v3 w/ rustls)

## Implementation
- `fetch_models(spec, auth_style, credential)` over `DiscoverySpec::Endpoint` from verified templates only — the app never invents catalog URLs.
- **Endpoint gate:** https required for all remote hosts; loopback http permitted for local mock fixtures only (threat T11).
- **Credential handling:** spec-declared `requires_auth` + missing credential → `CredentialRequired`; headers per template `AuthStyle` (`x-api-key` vs `Authorization: Bearer`).
- **Response hardening:** 10 s global timeout (ureq v3 agent config), 5 MB hard body cap (via `io::Read::take`), strict-JSON parse, documented `list_selector` path resolution, ≤1000 entries, per-entry validation (string `id`, length/charset), non-conforming entries skipped (not fatal), `capability_status` always `unknown` (F12 — never invented).
- Transport-error redaction strips query strings.
- **Mock API:** dependency-free one-shot loopback `TcpListener` HTTP fixtures (OpenAI-shape catalog with adversarial entries, 403 status, malformed JSON, wrong selector shape, header-echo server asserting the documented auth headers).

## Security review
- No shell; no user-controlled URLs (template-sourced only); caps prevent hostile-catalog DoS; error strings sanitized.
- ureq v3 (rustls) chosen over reqwest for a smaller dependency tree (threat T8); API migration (config-based timeout, `StatusCode` variant, `into_reader().take()`) documented in code.

## Tests (evidence)
- 9 discovery tests incl. 5 negative/adversarial paths; **Rust 137/137 + 7 integration**; clippy -D warnings clean; fmt clean.

## Acceptance criteria
- [x] Discovery interface with schema-validated mock API
- [x] Auth-style headers verified against a live echo fixture
- [x] Never auto-selects; capabilities stay unknown
