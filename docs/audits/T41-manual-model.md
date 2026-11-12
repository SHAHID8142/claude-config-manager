# T41 — Manual Model ID entry when discovery unavailable

**Date:** 2026-11-08 · **Result:** ✅ PASS · **Artifact:** App Model section (editable manual path)

## Implementation
- The Model field is now **editable** whenever no populated catalog is displayed: manual-entry providers (Z.AI — `DiscoverySpec::none`), custom providers, and empty catalogs. It remains read-only **only** while a fetched catalog is shown (T40 list path) and disabled with no provider selected.
- **Stale-model guard (security/correctness):** switching providers resets `selectedModel`, catalog results, and fetch state — a model id from another provider can never silently carry into an apply.
- Placeholder honesty preserved; never auto-filled.

## Security review
- Manual ids pass through the same `selectedModel` state later validated by `validate_model_id` at apply time (T43); no unvalidated write path exists.

## Tests (evidence)
- 3 new App tests: editable for zai + custom (with reset between), provider-switch reset, read-only while catalog displayed.
- Frontend **81/81** green.

## Acceptance criteria
- [x] Manual Model ID entry available when discovery is unavailable
- [x] No stale cross-provider model carry-over
