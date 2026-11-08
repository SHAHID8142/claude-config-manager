# T14 — Accessible form labels, keyboard navigation, focus states

**Date:** 2026-11-08 · **Result:** ✅ PASS

## Implementation
- **Focus states:** shared `focusRing` utility (`src/lib/focus.ts`) applied to every interactive control (select, inputs, both buttons, Advanced `summary`) — `focus-visible:ring-2` + offset, theme-aware. Keyboard-only users get a clearly visible indicator; mouse clicks don't.
- **Keyboard navigation:** DOM order equals visual order (single column), so tabbing follows the logical sequence; `<details>/<summary>` is natively keyboard-operable; native button/select semantics retained (no div-button antipatterns).
- **Labels & hints:** every field labelled (T12); now hints are programmatically linked via `aria-describedby` (FieldRow clones the control child and merges any existing describedby), so screen readers announce hints with the field.
- Shared `inputClass`/`secondaryButtonClass` to keep control styling consistent and future-proof.

## Security review
- UI-only change; no data/IPC.

## Tests (evidence)
- New a11y tests (3): hint↔control describedby linkage on multiple fields; focus treatment present on all 7 interactive elements (with `SUMMARY` exception noted and justified — native outline retained).
- `check:frontend` green; **14/14** tests.

## Acceptance criteria
- [x] Accessible labels (all controls)
- [x] Keyboard navigation (logical order + native semantics + focus indicator)
- [x] Focus states visible and theme-aware
