# T23 — Secret lifecycle + redaction tests

**Date:** 2026-11-08 · **Result:** ✅ PASS · **Artifacts:** `src-tauri/src/redact.rs`, `src-tauri/src/credential_service.rs`

## Implementation
- **Redaction guard** (`redact.rs`) — the central defense for threat T2:
  - `redact_secrets(text, secrets)`: replaces every secret occurrence with `[REDACTED]`; **empty/whitespace secrets ignored** (negative-proofed: an empty pattern would otherwise wipe the whole input); longest-first ordering so overlapping values (key + prefix) leave the strongest redaction.
  - `is_secret_shaped` / `strip_secret_shaped`: defensive scrubbing of *untrusted* text (upstream error strings) by credential-shape heuristics (known prefixes incl. `sk-ant-`, `sk-or-`, `nvapi-`, plus bearer-shaped tokens).
- **CredentialService** — lifecycle over any `SecretStore`: set (replace-in-place), has, delete (idempotent); service naming `com.claude-config-manager.<provider_id>` with strict provider-id validation; **every outward platform error is passed through redaction with the live secret set** before surfacing.

## Security review (checklist §Secrets)
- [x] Secret-in-error leak test: simulated OS error echoing the secret → surfaced message contains `[REDACTED]`, never the value
- [x] Empty-pattern destructive-redaction negative test
- [x] Per-provider service isolation verified (delete A leaves B)
- [x] No secret-shaped tokens survive `strip_secret_shaped`

## Tests (evidence)
- 11 new tests (6 redaction incl. 2 negative/adversarial, 5 lifecycle incl. leak simulation, isolation, validation).
- `cargo test` **47/47**; clippy -D warnings clean; fmt clean.

## Acceptance criteria
- [x] Replace/delete lifecycle implemented and tested
- [x] Redaction guard proven on log/error paths
