# T43 — Explicit model selection validation + Save & Apply integration

**Date:** 2026-11-08 · **Result:** ✅ PASS (1 defect found and fixed in-scope) · **Artifacts:** `src/lib/modelId.ts`,
`App.tsx`, `save_apply.rs`

## Implementation
- **The selected model is now actually applied.** `handleApply` sends
  `model: <validated, trimmed id>` in the `save_and_apply` payload; blank means `null`, never a guessed
  default. Until this task the Model section was display-only (audit M3).
- **Explicit validation before apply (PRD F03):** new `src/lib/modelId.ts` mirrors Rust
  `config_merge::validate_model_id` exactly — trim, non-empty, ≤ 256 chars, printable ASCII
  (`0x21..=0x7E`) only. One `modelValidation` result drives **both** the inline message and the
  Save & Apply gate, so the UI cannot offer an apply the backend would reject.
- **Honest failure state:** invalid input sets `aria-invalid="true"` on the field and renders a
  `aria-live="polite"` message that names the rule and says apply is disabled.
- **Defect fixed in scope (data-integrity):** Rust validated the *trimmed* model id but wrote the
  *raw* one, so `"  claude-sonnet-4-5  "` passed validation and landed padded in `settings.json`
  (Claude Code cannot resolve that id). `SaveApplyService::apply` now normalizes once
  (`trim` → `None` when empty) and validates the exact value it writes.
- Whitespace-only input is treated as "no model selected" (managed `model` left untouched) instead of
  raising an error the user cannot act on.

## Security review
- Validation is fail-closed on **both** sides; the Rust command remains the authority and re-validates
  every value, so a compromised renderer cannot bypass the gate.
- The id is written only into the managed `model` key; no env injection, no shell interpolation, no
  log/telemetry surface. Model ids are never used to build paths, commands or Keychain identifiers.
- No secret material touches this path: the payload still carries the key only to the CredentialService
  (T23 redaction unchanged).

## Tests (evidence)
Rust (`save_apply`):
- new `model_input_is_normalized_before_write`: padded id → written trimmed; whitespace-only → managed
  model preserved; invalid characters (`"bad model\n"`) → `InvalidInput` **and the file is byte-identical
  afterwards** (nothing written).
- existing `full_pipeline_happy_path` / `invalid_provider_and_endpoint_rejected_before_any_write` still
  green.

Frontend:
- `src/lib/modelId.test.ts` (6 cases): valid ids, trimming, empty/whitespace-only, embedded
  space/control chars, non-ASCII, 256-cap boundary (`256 ok` / `257 rejected`).
- `App.test.tsx` → `Model selection validation + Save & Apply integration (T43)` (5 cases): typed id is
  sent; no selection → `null`; padded id sent trimmed; invalid id disables apply + shows the message +
  `save_and_apply` never called + fixing it re-enables; catalog-selected id is sent with the preset
  `baseUrl`.

Gates (local, this commit):
- `npm run lint` ✓ · `format:check` ✓ · `typecheck` ✓ · `test` → **98/98 passed** (10 files)
- `npm run build` ✓ (240.17 kB js)
- `cargo fmt --check` ✓ · `cargo clippy -- -D warnings` ✓ · `cargo test` → **144 passed, 1 ignored + 7
  integration = 151** ✓

## Acceptance criteria
- [x] Model selection is validated before applying (client + server, same rules)
- [x] Save & Apply writes the chosen model to `settings.json`
- [x] No model chosen ⇒ no model written and no invented default
- [x] Negative case covered: invalid id blocks apply and never reaches the backend
- [x] Trimming defect found during implementation fixed and regression-tested

## Known limitations
- Validation checks **shape**, not **existence**: a well-formed but nonexistent model id is accepted
  (it would fail at request time in Claude Code). Verifying existence requires a per-provider catalog
  fetch at apply time, which would add a network dependency to a local write path — deliberately out
  of scope; disclosed in the UI copy as "manually entered".
- Cross-provider compatibility (does this model support Claude Code tool use?) remains unverified by
  design (PRD F12); capability labels stay `unknown` (T42).
- The success banner does not echo the model; the "Active configuration" panel does, because
  `read_status` re-runs after `applyOutcome` (T34 behaviour).
