# T16 — Versioned provider template schema

**Date:** 2026-11-08 · **Result:** ✅ PASS · **Artifact:** `src-tauri/src/provider_catalog.rs`

## Implementation
- `ProviderTemplate` wire shape with `schema_version: u32` gate (build understands v1; unknown versions rejected — forward-compat, never guessed).
- Enums per T02 matrix: `AuthStyle` (`api_key` → x-api-key, `bearer_token` → Authorization Bearer), `Protocol` (`anthropic_native`/`anthropic_compat`/`open_ai_only`), `BridgeRequirement` (ADR-001), `DiscoverySpec` (`none`/`endpoint{url, requires_auth, list_selector}`), `CapabilityStatus` (`verified{source_url}`/`unknown` — never fabricated).
- Semantic validation: kebab-case id, HTTPS-only URLs (threat T11), protocol↔bridge consistency (OpenAI-only ⇒ bridge required; non-native ⇒ base URL mandatory), non-empty names.
- Dependency: `url` crate (well-maintained canonical Rust URL parser).

## Security review
- HTTPS-only enforced at parse time for template-declared endpoints (catalog presets cannot smuggle http:// or garbage URLs).
- No network in the catalog module (boundary respected); capability claims carried verbatim with source citation.

## Tests (evidence)
- 9 catalog tests: valid compat parse; **negative cases**: unknown schema_version, missing field, http:// base URL, bad id characters, OpenAI-only without bridge, compat without base URL, non-https discovery URL, URL validator garbage inputs.
- `cargo test` **13/13** (incl. existing contract tests), clippy -D warnings clean, fmt clean.

## Acceptance criteria
- [x] Versioned schema with direct/bridge protocol metadata
- [x] Parse + validate with negative cases
- [x] No capability fabrication (status carried verbatim)
