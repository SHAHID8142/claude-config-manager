# T21 — OS credential vault interface with mock tests

**Date:** 2026-11-08 · **Result:** ✅ PASS · **Artifact:** `src-tauri/src/secret_store.rs`

## Implementation
- `SecretStore` trait (`set_secret`/`get_secret`/`delete_secret`), storage-agnostic, blocking-by-design for Tauri command contexts (documented).
- `SecretError` taxonomy: `Platform{message}` (pre-redacted by platform store, contract forbids echoing secret material), `ItemNotFound`, `InvalidIdentifier`. Deletion is idempotent (a promise the secret is gone, not a report about prior state) — documented.
- Identifier validation: non-empty, ≤256 chars, no NUL — shared by all stores.
- `MockSecretStore`: Mutex-HashMap-backed, thread-safe, serves tests and mock transports. Empty secrets are never stored (clear = delete).
- Service naming convention: `com.claude-config-manager.<provider>` (tests use `.test-provider`).

## Security review
- No secret ever travels in an error path (asserted by test); identifiers are structural, not secrets.
- Empty-secret rejection prevents accidental zero-value writes that could later be misread as "unset".

## Tests (evidence)
- 8 tests: roundtrip set/get/delete, replace-no-duplicate, account isolation, idempotent delete, identifier negatives (empty/oversized), empty-secret rejection, error display carries no secret material, 8-thread concurrency.
- `cargo test` **31/31**; clippy -D warnings clean; fmt clean.

## Acceptance criteria
- [x] Storage-agnostic vault interface
- [x] Mock implementation with positive + negative + concurrency tests
- [x] Keychain-backed store follows in T22 behind this trait
