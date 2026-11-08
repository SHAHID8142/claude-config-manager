# T40 — Searchable model list; NEVER auto-select

**Date:** 2026-11-08 · **Result:** ✅ PASS (1 focused fix cycle) · **Artifacts:** `src/components/ModelList.tsx` + App integration

## Implementation
- **ModelList**: search filter (case-insensitive on id + display name), scrollable option list, per-model `capabilities unknown` chip (F12), honest counters ("N of M models shown") and empty-filter state.
- **Selection semantics (PRD F04):** a model becomes selected ONLY via explicit click; fetch always starts with zero selections and a re-fetch resets any prior selection (no cross-provider inheritance — asserted). The Model field mirrors the selection read-only while a catalog is loaded.
- **A11y:** filter input labelled (sr-only label + describedby hint), buttons carry `role="option"` + `aria-selected`, container labelled for AT; focus-ring on all options.
- One fix cycle: clicking the `<li>` bypassed the button handler → `role="option"` moved onto the button; RTL `{selected: true}` aria-string gotcha replaced with explicit `[aria-selected='true']` checks.

## Security review
- Display-only component; model ids flow to state for later Save & Apply validation (T43); no fabrication of capability data.

## Tests (evidence)
- 3 component tests: render+chips, filtering incl. empty state, explicit-click-only selection with exactly-one-selected invariant.
- 2 App tests: click-select mirrors into the Model field; re-fetch resets selection.
- Frontend **78/78** green.

## Acceptance criteria
- [x] Searchable list, selection only by explicit action
- [x] Nothing auto-selected on fetch or filter
- [x] Capability chips remain "unknown"
