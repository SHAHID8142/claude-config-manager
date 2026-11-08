# T28 — Minimal targeted configuration merge + schema validation

**Date:** 2026-11-08 · **Result:** ✅ PASS · **Artifact:** `src-tauri/src/config_merge.rs`

## Implementation
- `DesiredSettings` (all `Option`, default = touch nothing) over the T02 verified write set: `model` + 7 `env.ANTHROPIC_*` keys; registries `MANAGED_TOP_KEYS`/`MANAGED_ENV_KEYS` with a consistency test.
- `validate()` gate BEFORE any merge: model-ID shape (printable ASCII, no whitespace/control, ≤256), https-only `ANTHROPIC_BASE_URL` (local-https allowed, per T19 classification), auth values non-empty/control-free/≤4096.
- `apply_desired(current, desired)`: clone-and-patch — unmanaged top-level keys AND unmanaged `env` keys pass through verbatim (F02); managed keys replaced only when `Some`; **`None` = untouched, never removal** (settings set-not-remove per T02); corrupted existing `env` (non-object) refused with an explicit error; new `env` created when missing/null; non-object current rejected.
- `ConfigError::ValidationFailed` variant added to the shared taxonomy.

## Security review
- Pre-write validation means a rejected value can never produce a partial document.
- Merging never deletes; the user's pre-existing proxy env (observed on this machine, T02 §7) is preserved unless explicitly managed — override warnings arrive at read-back (T34).

## Tests (evidence)
- 12 tests: empty-doc write, unrelated top-level preservation, **unrelated-env preservation**, none-means-untouched, managed replacement, invalid model ID (negative), http base URL (negative), local https OK, non-object current (negative), **corrupted-env refusal** (negative), null-env creation, registry consistency.
- `cargo test` **81/81**; clippy -D warnings clean; fmt clean.

## Acceptance criteria
- [x] Minimal targeted merge, schema-validated
- [x] Unrelated settings preserved at both levels
- [x] Negative cases: invalid values, corrupted shapes
