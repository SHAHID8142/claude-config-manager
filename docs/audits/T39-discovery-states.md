# T39 — Refresh, loading, timeout, error states for model discovery

**Date:** 2026-11-08 · **Result:** ✅ PASS · **Artifact:** App model section (states + fetch flow)

## Implementation
- **Fetch models** `LoadingButton` in the Model section: pending state (`aria-busy`, "Fetching…"), disabled without a selected preset (custom providers use manual entry — honest hint), retry always available after failure.
- **States:** loading → success (count displayed, selection UI explicitly deferred), empty catalog (honest "no models — manual entry arrives next task"), typed error banner (`StatusBanner`, dismissible) — the backend's 10 s timeout/HTTP-status/credential errors surface verbatim (e.g. "catalog endpoint returned HTTP 403").
- **Never auto-selects (PRD F04):** after a successful fetch the model input remains disabled/empty — asserted in tests.
- Re-fetch per provider switch resets state (fetched-for tracking prevents stale results).
- Credential flow: the typed key (if present) is passed; otherwise the backend resolves the stored Keychain credential only for auth catalogs.

## Security review
- No new data surfaces; error banners carry backend messages already sanitized by the T37 redaction.

## Tests (evidence)
- 3 new App tests: gating + success count + no-auto-select, failure banner + retry availability, empty-catalog state.
- Frontend **73/73**; Rust unchanged (142+7, no backend change this task).

## Acceptance criteria
- [x] Refresh/loading/timeout/error states wired and tested
- [x] Nothing auto-selected at any point
