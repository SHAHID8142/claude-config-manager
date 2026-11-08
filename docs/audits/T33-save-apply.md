# T33 — Save & Apply for ONE verified direct provider (Anthropic)

**Date:** 2026-11-08 · **Result:** ✅ PASS (2 focused fix cycles, within budget) · **Artifacts:** `src-tauri/src/save_apply.rs`, `save_and_apply` command, TS contract + App wiring

## Implementation
- **`SaveApplyService`** (injectable home/store/dirs; production wiring: `$HOME` + macOS Keychain): validate inputs → store credential (Keychain) → generate + install helper script (T32, 0700) → snapshot mtime → read settings (**malformed sources refused — never overwritten**) → merge (T28) → backup **only when a file exists** (first-run has nothing to back up) → atomic write with conflict detection (T30) → read-back → `ApplyOutcome{settingsPath, backupFileName, helperPath, appliedKeys}`.
- **Command layer:** `save_and_apply` Tauri command mapping `ApplyError` → typed `CommandError` (messages pre-redacted by the service — leak test asserted).
- **Writer evolution:** first-run creation allowed (Missing target created atomically; stale-mtime creation refused; failed first-write cleans up instead of "restoring" an empty file).
- **TS/App:** typed `saveAndApply` contract; `ProviderSummary.directBaseUrl` added (catalog stays source of truth — compat presets pass their verified base URL, custom providers pass the user endpoint, native passes none); Save & Apply is a `LoadingButton` (aria-busy pending state), success/error `StatusBanner`s, and an honest disclosure line ("key stored in Keychain via apiKeyHelper — never written into settings.json").
- Button gating: requires provider selected **and** non-empty key; custom path requires endpoint.

## Security review
- Failure-after-secret paths leak nothing (dedicated test with the secret in scope during a forced late failure).
- UI never echoes the key; the mock-invocation test asserts the transport receives it only inside the typed args.

## Tests (evidence)
- Rust **110/110** + 7 integration: happy path (managed keys written, credential stored, helper 0700, backup created, appliedKeys reported), preservation across apply, **malformed source never overwritten** (byte-identical + no backup of broken file), no-credential blocks helper mode, invalid inputs rejected before ANY write, secret-leak guard.
- Frontend **60/60**: preset apply flow (button gating → success banner), error banner without key echo, compat/custom base-URL pass-through.
- clippy -D warnings clean; fmt clean.

## Fix cycles (2 of 3)
1. Writer refused first-run creation → added Missing-target creation with correct rollback semantics.
2. Two test-expectation bugs from the new seeded fixture (env preservation + existing-file assertions) — corrected to assert actual correct behavior.

## Acceptance criteria
- [x] End-to-end save→store→helper→merge→backup→atomic write→verify on fixtures
- [x] One verified provider (Anthropic) wired through the UI
- [x] Failure and first-run paths covered
