# T34 — Read-back status + override warnings

**Date:** 2026-11-08 · **Result:** ✅ PASS (1 clippy fix, cycle 1) · **Artifacts:** `src-tauri/src/config_status.rs`, `read_status` command, App status panel

## Implementation
- **`read_effective_status`** — reads the live settings file (missing → honest first-run state; malformed → error surfaced, not a zero status) and extracts: model, env model/base-url (base URL disclosed — not a secret), credential presence (auth token / api key / helper path).
- **Override warnings straight from the T02 precedence matrix (PRD F06):**
  - `HelperShadowedByEnv` — apiKeyHelper present but plaintext env credentials outrank it;
  - `PlaintextKeyInEnv{which}` — plaintext credential material in settings.json (name only, never the value);
  - `ModelConflict` — `model` vs `ANTHROPIC_MODEL` disagree (env wins).
- **Command:** `read_status` (production: `$HOME` + default locator).
- **App:** Active-configuration section now renders the live status (model, endpoint, credential mode) with warnings list; header mode label derived honestly (`direct (Keychain helper)` / `direct (plaintext env)` / `not configured`); status refreshes after each successful apply.
- TS wire mirror matches Rust's externally-tagged enum serialization exactly.

## Security review
- Warning payloads carry key NAMES and config values only — never credential values (asserted in tests).
- Endpoint disclosure is deliberate (base URL is not secret material; the user's pre-existing proxy case is exactly what F06 wants surfaced).

## Tests (evidence)
- Rust **117/117** (7 new: first-run, clean helper config, shadowing, plaintext flagging, conflict, no-false-conflict, malformed error).
- Frontend **65/65** (5 new: first-run state, shadowing warnings, model conflict, endpoint disclosure, post-apply refresh; call-order assertion updated for the status effect).
- clippy -D warnings clean; fmt clean.

## Acceptance criteria
- [x] Read-back status of effective config
- [x] Override/uncertainty warnings derived from verified precedence
- [x] Never-echo-secret discipline
