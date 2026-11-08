# T46 — Verified model-role mapping: none shipped; failover forbidden

**Date:** 2026-11-08 · **Result:** ✅ PASS (decision + enforcement tests) · **Artifact:** `save_apply.rs`,
`docs/decisions/ADR-002-no-model-failover.md`

## Investigation (grounded in official docs)
`docs_lookup` on Claude Code (CHANGELOG + settings reference) established:
- `ANTHROPIC_DEFAULT_{HAIKU,SONNET,OPUS}_MODEL` **pin the `haiku`/`sonnet`/`opus`/`opusplan` aliases** to a
  different model; the documented case is pointing 3P (Bedrock/Vertex) setups at a newer Haiku.
- The alias defaults belong to Anthropic and **move between releases**.
- **No** documented fallback/failover chain exists for model selection.

## Decision (ADR-002)
- **No mapping table is shipped.** Anything hardcoded would be unverified for third-party providers and
  would rot on every release. Users get Anthropic's default unless they explicitly pin an id (T45).
- **No automatic failover, ever.** A rejected model id fails the apply closed; nothing is substituted.

## Enforcement (new regression tests)
- `apply_never_writes_a_second_model_key_or_a_failover_chain` — asserts `model` is a single string,
  `env.ANTHROPIC_MODEL` remains **absent** (no second model source that could shadow the managed key),
  and each role key is a single id rather than a list/chain.
- `failed_apply_keeps_the_previous_model_instead_of_failing_over` — after a successful
  `good-model` + sonnet pin, a follow-up apply with `"bad model"` is rejected and the file is
  **byte-identical** (prior model and role pin intact). No "safe model" appears.
- Existing T45 test `absent_or_blank_role_overrides_leave_existing_values_untouched` covers "no invented
  mapping".

## Security review
- Removing failover removes a class of silent-redirect risk: a compromised or buggy fallback could send
  requests (and the credential) to an unintended model/endpoint. Fail-closed is the safer default.
- No new code path, no new I/O, no credential surface change.

## Tests (evidence)
- `cargo test failover` → `apply_never_writes_a_second_model_key_or_a_failover_chain` **ok**
- `cargo test failed_apply_keeps` → **ok**
- Full suite: fmt ✓ · clippy `-D warnings` ✓ · `cargo test` → see progress row (frontend untouched).

## Acceptance criteria
- [x] Mapping added only if verified → verified that none is justified; none added
- [x] No automatic failover — enforced by tests, not just documented
- [x] Decision recorded with its reasoning and consequences (ADR-002)

## Known limitations
- Users who *want* alias pinning must know the target model id themselves; the app deliberately does not
  suggest one.
- Role pinning semantics for non-Anthropic providers (does DeepSeek honour `ANTHROPIC_DEFAULT_*`?) remain
  unverified — Claude Code passes them to the endpoint, the provider may ignore them. Capability status
  stays `unknown` (F12).
