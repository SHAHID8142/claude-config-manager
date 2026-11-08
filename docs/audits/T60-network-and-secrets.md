# T60 — Localhost exposure, download tampering and secret-leak regression suite

**Date:** 2026-11-08 · **Result:** ✅ PASS — **5 real defects found and fixed** · **Artifact:**
`src-tauri/tests/secret_and_network_surface.rs` (11 cases) + fixes in `provider_catalog.rs`,
`model_discovery.rs`, `config_merge.rs`, `redact.rs`

## Coverage and what it proved

### 1. Localhost exposure
- Wildcard binds (`0.0.0.0`, `[::]`) and the link-local metadata address (`169.254.169.254`) are refused
  outright by both `classify_endpoint` and discovery's `validate_endpoint`.
- Private LAN ranges (`192.168.*`, `10.*`, `172.16.*`) are never classified `Local` (so they can never be
  mistaken for a local gateway) — they are remote, and discovery refuses plain `http` to them.
- Discovery accepts plain `http` **only** for true loopback (`127.0.0.1`, `localhost`).

### 2. Download tampering — honest scope statement
No bridge/download code exists (T47–T58 blocked on consent), so there is nothing to verify for pinned
hashes/signatures. Instead of faking coverage, the suite asserts the invariant that **does** exist:
- presets are compile-time embedded, so planting a tampered `deepseek.json` on disk changes nothing;
- a guard test fails the build if a download path (`curl`/`wget`/`reqwest::blocking::get`) appears in
  `lib.rs` before the integrity tests exist.

### 3. Secret leakage
- Discovery errors never echo the credential (500-path tested with a real fake key).
- The generated helper script contains the Keychain lookup, never a key.
- The merge layer writes `apiKeyHelper`, never a plaintext credential.
- `redact_secrets` removes exact matches; `is_secret_shaped`/`strip_secret_shaped` remove
  shape-matched secrets without over-redacting ordinary model ids.

## Defects found (each now guarded by a failing-first test)
1. **Secret-shaped model ids were accepted.** `sk-ant-api03-…` passes "printable ASCII", so a pasted key
   would have been written to `settings.json` in plaintext (and into backups). `validate_model_id` now
   refuses credential-shaped values.
2. **`validate_endpoint` (discovery) accepted embedded credentials** (`https://user:pass@…`) while
   `classify_endpoint` refused them — two validators, two rules. Both now refuse.
3. **Wildcard `0.0.0.0` was accepted as a remote endpoint**, and `http://0.0.0.0` was reported as merely
   "Insecure" because the scheme branch returned before any host check. Host refusals now run first.
4. **`key=value` secrets were not redacted.** `strip_secret_shaped("token=sk-ant-…")` left the key intact
   because the token did not *start* with a known prefix. Detection now also inspects the value after the
   last `=`/`:`, and stripping keeps the key name while removing the secret.
5. **No wildcard/link-local check at all in discovery**, so a malformed custom catalog URL could point at
   the cloud metadata service. Same rules now apply as in the UI classifier.

## Tests (evidence)
- `cargo test --test secret_and_network_surface` → **11 passed**
- Full suite: **156 unit (+2 ignored live) + 7 adversarial + 14 malformed + 11 secret/network = 188**
- `cargo fmt --check` ✓ · `cargo clippy --all-targets -- -D warnings` ✓ · frontend untouched (110/110)

## Acceptance criteria
- [x] Localhost/wildcard exposure covered by tests and refused by code
- [x] Download-tampering gap stated honestly + supply-chain invariant tested
- [x] Secret leak tests across discovery, helper script, merge, redaction
- [x] Findings fixed rather than documented away

## Known limitations
- Bridge download integrity (pinned version, SHA-256, signature, no-execute) is **NOT TESTED** — the code
  does not exist yet. The guard test forces that work to bring tests with it.
- TLS-level attacks and DNS rebinding are out of scope; the app relies on the platform TLS stack.
- `/16`-style link-local checks are prefix-based on the host string, not a full IP parse; equivalent for
  the addresses that matter here but not a general IP classification.
