# T45 — Advanced settings panel with default-no-override behaviour

**Date:** 2026-11-08 · **Result:** ✅ PASS (1 wiring bug caught by test and fixed) · **Artifacts:**
`src/lib/roles.ts`, `src/components/AdvancedSettings.tsx`, `App.tsx`, `save_apply.rs`, `config_status.rs`,
`lib.rs`, `ipc.ts`

## What was built
The three Claude Code model-role mappings are now editable — and **only** editable on purpose:

| Role | Managed key |
|---|---|
| Haiku | `ANTHROPIC_DEFAULT_HAIKU_MODEL` |
| Sonnet | `ANTHROPIC_DEFAULT_SONNET_MODEL` |
| Opus | `ANTHROPIC_DEFAULT_OPUS_MODEL` |

- Panel is **collapsed by default**; expanding it is the only way to reach the fields.
- Blank field ⇒ **no override**: the managed key is left exactly as it is. Nothing is invented, nothing
  is cleared silently, and no mapping is "helpfully" guessed (PRD F05).
- Each row displays the **currently configured** value (`Current: <id>` or `Current: no override
  (Claude Code default)`), fed by `read_status`. The panel cannot imply "not set" when an override is
  already active.
- Validation reuses the T43 rules (same `validate_model_id` on the Rust side, TS mirror for instant
  feedback); invalid input sets `aria-invalid`, gets a `aria-live="polite"` message and disables apply.

## Backend plumbing
- `save_apply::RoleOverrides { haiku, sonnet, opus }` (all `Option<String>`, `Default` = all `None`).
- `SaveApplyService::apply_with_roles(...)` reuses the whole fail-closed pipeline; `apply(...)` delegates
  with `RoleOverrides::default()`, so no existing caller or test changed behaviour.
- Role values are trimmed (`"  fast  "` → `fast`), blank → `None`, and validated **before any write**;
  a rejection leaves `settings.json` byte-identical.
- `save_and_apply` gained `role_haiku` / `role_sonnet` / `role_opus` parameters (null = no override).
- `EffectiveStatus` gained `env_default_haiku|sonnet|opus` so the UI can tell the truth about state.

## Bug found by the new tests (and fixed)
The Save & Apply button carried its **own** `disabled` expression (`selected === null || apiKey empty ||
modelInvalid`) instead of using the single `canApply` gate. Role-invalid input therefore left the button
enabled — the UI would have offered an apply the pipeline rejects. Now `disabled={!canApply}`: one gate,
one source of truth.

Also restructured for lint/a11y hygiene: pure role logic lives in `src/lib/roles.ts` so the component file
exports only a component (`react-refresh/only-export-components` errors were fixed properly rather than
suppressed).

## Security review
- No new secret surface: role values are model identifiers written to the same managed `env` block.
- Merge semantics unchanged (`None` = untouched; the app still cannot *remove* a managed key — T28 rule).
- Validation is duplicated client/server as before; Rust remains authoritative.
- The panel is keyboard reachable, uses a real `<button aria-expanded aria-controls>` disclosure and
  `role="group" aria-label` for the panel — no invented ARIA roles (M9 lesson).

## Tests (evidence)
Rust:
- `role_overrides_are_written_when_the_user_sets_them` — padded haiku stored trimmed, opus absent stays
  absent, unrelated `CLAUDE_CODE_X` preserved.
- `absent_or_blank_role_overrides_leave_existing_values_untouched` — pre-existing opus override survives a
  blank apply; no new keys appear.
- `invalid_role_override_is_rejected_before_any_write` — `"bad model with spaces"` → `InvalidInput`,
  file byte-identical.
- `role_overrides_are_reported_and_default_to_none` (status) — sonnet reported, haiku/opus `None`;
  missing file → all `None`.

Frontend:
- `AdvancedSettings.test.tsx` (8): collapsed by default + `aria-expanded`/`aria-controls`; three fields
  revealed with "No override" placeholders; toggle callback; `onChange` per role; current value shown;
  invalid field flagged with `aria-invalid` + error id; blank ⇒ `{}`; only validated+trimmed values sent.
- `App.test.tsx` → `Advanced role overrides + Save & Apply (T45)` (4): panel collapsed and
  `roleHaiku/Sonnet/Opus` null by default; filled roles sent trimmed with untouched roles null; invalid
  role disables apply, shows the message, recovers when fixed; honest "no override" state rendered.

Gates: `npm run lint` ✓ · `format:check` ✓ · `typecheck` ✓ · **110/110 TS** ✓ · `build` ✓ ·
`cargo fmt --check` ✓ · `cargo clippy -- -D warnings` ✓ · **154 passed, 2 ignored + 7 integration = 163** ✓

## Acceptance criteria
- [x] Advanced panel exists, collapsed by default, keyboard/AT operable
- [x] Default behaviour is genuinely "no override" (proved by tests on both layers)
- [x] User-set roles are written to the documented keys
- [x] Invalid input blocked before write, with honest UI state
- [x] Current effective values surfaced instead of implied

## Known limitations
- Clearing a role back to "no override" is **not** possible through this panel: the merge contract only
  sets keys (T28), so an existing override is replaced, not removed. Removing requires editing
  `settings.json` or restoring a backup. Documented, not hidden.
- Role overrides apply to the selected provider's model ids without compatibility checking — same
  disclosed limitation as T42/T43 (capability stays `unknown`).
- No UI yet explains *when* Claude Code uses each role; the hint names the env var only.
