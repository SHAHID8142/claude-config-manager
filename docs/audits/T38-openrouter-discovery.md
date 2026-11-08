# T38 — Discovery for one documented provider (OpenRouter)

**Date:** 2026-11-08 · **Result:** ✅ PASS · **Artifacts:** `src-tauri/src/model_discovery_service.rs`, `discover_models` command, TS contract

## Implementation
- `discover_for_provider(provider_id, explicit_credential, store)` — resolves the template's `DiscoverySpec` + `AuthStyle`, consults the SecretStore **only when the catalog requires auth** (public catalogs never trigger credential reads), falls back to an explicit credential, then runs the T37 fetch pipeline.
- Error taxonomy preserved: unknown provider / `DiscoverySpec::None` → `InvalidEndpoint` (manual-entry providers honestly refuse discovery); missing credential → `CredentialRequired`; network failures → `Transport`.
- **Command:** `discover_models` (production resolves via the macOS Keychain when no explicit credential passed).
- **TS contract:** `discoverModels(providerId, credential?)`.

## Live verification (opt-in test, run manually)
- `cargo test live_openrouter -- --ignored --nocapture` → **"live OpenRouter catalog: 464 models, first: liquid/lfm-3-1.7b" — PASSED** against the documented public endpoint (no auth), matching the earlier curl probe (HTTP 200, `data[]` shape). CI stays network-free (test `#[ignore]`d).

## Security review
- Keychain consulted only for auth-requiring catalogs; no secret enters error paths; template-sourced URLs only.

## Tests (evidence)
- 5 new service tests (unknown provider, manual-entry refusal, OpenRouter shape mapping, Keychain resolution proves non-`CredentialRequired` failure, missing-credential → `CredentialRequired`); live test recorded above.
- Rust **142/142 + 7 integration**; frontend **70/70**; clippy -D warnings clean; fmt clean.

## Acceptance criteria
- [x] Discovery wired for one documented provider (OpenRouter)
- [x] Real-network verification recorded; CI remains offline
