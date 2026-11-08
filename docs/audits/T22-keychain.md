# T22 — macOS Keychain-backed vault + error handling

**Date:** 2026-11-08 · **Result:** ✅ PASS (after one focused dependency fix, cycle 1 of 3)

## Implementation
- `KeychainStore` implementing `SecretStore` via the `keyring` crate against the real macOS keychain; error mapping reduces every keyring error to a redaction-safe `SecretError` (`NoEntry`→`ItemNotFound`; refusals → generic "keychain access refused by the OS" — internal OS detail never surfaces, asserted by test; `TooLong`→`InvalidIdentifier`).
- **Key finding (fix cycle 1):** `keyring` v3's macOS backend uses the **legacy `SecKeychain` file APIs, which silently no-op on macOS 26** (set returned Ok while nothing persisted — verified via probe + `security` CLI, sandbox-independent). Upgraded to **keyring v4** (`apple-native-keyring-store` backend, modern data-protection keychain). Verified: cross-process persistence (item read back by a fresh process), full single-instance lifecycle set→get→delete→none.
- Real-keychain round-trip test (`keychain_round_trip_real`, macOS-only): unique `ccm-selftest-<nanos>` account, fake secret, set/get/delete/verify-gone, **passes against the real keychain** (36/36).
- Probe utility `examples/kctest.rs` kept for manual verification with fresh-account-per-run design.

## Security & OS-integration review
- **macOS authorization prompt observed (documented, not bypassed):** accessing an item created by a *different* process instance (e.g., deleting `ccm-selftest-probe` from a new run) blocks on an OS ACL prompt. Consequence recorded: our store always accesses only items **this app instance created** under `com.claude-config-manager.<provider>` — the normal app flow never hits cross-instance ACL prompts. A leftover fake probe item (`ccm-selftest-probe`, value `sk-test-PROBE-123`) may remain in the keychain and can be removed via Keychain Access; touching it again will prompt — by design.
- Error redaction verified at the mapping layer (no OS internals, no secret material in errors).

## Tests (evidence)
- `cargo test` **36/36** (incl. real-keychain round-trip + idempotent-delete-on-missing real test).
- `cargo clippy -D warnings` clean; `cargo fmt --check` clean.
- Manual probe run recorded above (set/get/delete/get-none, exit 0).

## Acceptance criteria
- [x] Keychain-backed vault functional on real macOS
- [x] Error handling mapped + redacted
- [x] Real round-trip verified (not simulated)
