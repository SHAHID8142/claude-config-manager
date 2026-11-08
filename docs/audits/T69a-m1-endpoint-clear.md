# T69a — M1 fix: switching providers no longer inherits the previous endpoint

**Date:** 2026-11-08 · **Result:** ✅ PASS — release-blocking defect fixed · **Severity:** critical
(credential misrouting) · **Artifacts:** `config_merge.rs`, `save_apply.rs`

## The defect
The merge contract could only **set** managed keys (`None` = "leave untouched"), so switching from a
compat provider (DeepSeek/OpenRouter/Z.AI) to **native Anthropic** passed `base_url = None` and left the old
`env.ANTHROPIC_BASE_URL` in place. Claude Code would then send requests — **with the Anthropic key** — to
the previous third-party endpoint. Reported as M1 in the original read-only audit and still present at T67.

Reproduced before the fix: apply DeepSeek (`ANTHROPIC_BASE_URL=https://api.deepseek.com/anthropic`), then
apply `anthropic` with no base URL → the env key survived.

## The fix
- `DesiredSettings.clear_env_base_url: bool` — an explicit removal instruction, because `None` already means
  "untouched" and overloading it would silently delete user values in other flows.
- `apply_desired` removes `ANTHROPIC_BASE_URL` when the flag is set (before applying any env writes), and now
  also handles the "only a removal, no writes" case — previously the env block was skipped entirely when
  `env_writes` was empty.
- `SaveApplyService` sets `clear_env_base_url = base_url.is_none()`, i.e. **"the user selected a provider
  with no endpoint override" ⇒ ensure there is none**. This is the app's own key, so removal is correct
  ownership, not data loss.
- Removal is scoped to exactly one managed key; nothing else in `env` is touched (proved by test).

## Security impact
Before: a user switching back to Anthropic could have every request (and its API key) sent to the previous
provider's endpoint — the single worst failure mode this app can have. After: the managed endpoint is
explicitly removed, so Claude Code falls back to its real Anthropic endpoint.

## Tests (evidence)
- `switching_to_native_clears_the_previous_provider_endpoint` — sets DeepSeek endpoint, switches to native,
  asserts `ANTHROPIC_BASE_URL` is **gone**, unrelated `CLAUDE_CODE_X` preserved, model updated.
- `clearing_an_absent_endpoint_is_a_no_op` — a settings file without an endpoint is left untouched
  (`{"KEEP":"1"}` unchanged).
- Full suite: **158 unit (+2 ignored) + 7 adversarial + 7 crash + 6 lifecycle + 14 malformed + 11
  secret/network = 203** · `cargo clippy --all-targets -- -D warnings` ✓ · TS 120/120 ✓

## Acceptance criteria
- [x] Defect reproduced and fixed at the contract level (not papered over in the UI)
- [x] Only the managed key is removed; user env untouched
- [x] Regression tests cover both the switch and the no-op case
- [x] No silent behaviour change elsewhere: `None` still means "untouched" for every other field

## Known limitations
- Only `ANTHROPIC_BASE_URL` gained a removal path. Other managed keys still cannot be removed by the app
  (documented contract, T28); clearing them remains a manual edit.
- The UI does not yet show a "removed the previous endpoint" confirmation; the Active configuration panel
  simply stops displaying an endpoint, which is accurate but not explanatory.
