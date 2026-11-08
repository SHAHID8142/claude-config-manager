# ADR-002 — No shipped model-role mapping, no automatic model failover

**Date:** 2026-11-08 · **Task:** T46 · **Status:** Accepted

## Context
Backlog T46 asks for "only verified Claude Code model-role mapping if needed; no automatic failover".

Claude Code's own documentation (CHANGELOG + settings reference, verified 2026-11-08) describes:

- `ANTHROPIC_DEFAULT_HAIKU_MODEL`, `ANTHROPIC_DEFAULT_SONNET_MODEL`, `ANTHROPIC_DEFAULT_OPUS_MODEL`
  **pin the `haiku` / `sonnet` / `opus` / `opusplan` aliases** to a different model than the built-in
  default (the documented use case is pointing 3P setups at a newer Haiku).
- The built-in default for each alias is chosen by Anthropic and **changes with releases**.
- No fallback/failover chain is documented anywhere for model selection.

## Decision
1. **Ship no role→model mapping table.** The app exposes the three pinning variables as optional,
   blank-by-default overrides (T45) and otherwise leaves Claude Code's own defaults intact. Hardcoding
   ids would drift on every release and would be unverifiable for third-party providers.
2. **Never fail over.** No automatic substitution, retry-with-another-model, or hidden "safe model". If a
   model id is rejected the apply fails closed and the previous configuration stays byte-identical.

## Consequences
- Users get exactly the model they typed, or Anthropic's default — never a guess.
- `env.ANTHROPIC_MODEL` is never written by the app, so there is no second model source that could
  shadow or contradict the managed `model` key (T02 precedence stays intact).
- A user who wants alias pinning must type the id (T45); the app will not populate it for them.

## Enforcement
Regression tests in `save_apply` (fail if either rule is broken):
- `apply_never_writes_a_second_model_key_or_a_failover_chain` — `model` is a single string,
  `env.ANTHROPIC_MODEL` stays absent, role keys are single ids.
- `failed_apply_keeps_the_previous_model_instead_of_failing_over` — a rejected apply leaves the file
  byte-identical and keeps the prior model and role pin.
- `absent_or_blank_role_overrides_leave_existing_values_untouched` (T45) — no invented mapping.
