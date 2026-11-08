# T15 — Loading, error, success feedback components

**Date:** 2026-11-08 · **Result:** ✅ PASS

## Implementation
- **`StatusBanner`** — four severities (info/success/warning/error) with theme-aware palettes; `role="alert"` + `aria-live="assertive"` for errors (interruptive announcement), `role="status"` + `polite` otherwise; visually-hidden severity prefix for screen readers; optional detail line; optional dismiss control.
- **`LoadingButton`** — pending state: `aria-busy="true"`, disabled while pending, pending label override, spinner with `motion-reduce:animate-none` (respects reduced-motion preference, per TEST-STRATEGY a11y rules).
- Components are deliberately **not yet wired into App** — they attach to real flows in T19 (validation warnings), T33 (Save & Apply), T39 (discovery states). Wiring them to fake flows now would violate the honest-state policy from T12.

## Security review
- Presentation-only; messages are plain strings passed by callers (redaction duty stays with callers; enforced when wired in T57).

## Tests (evidence)
- `StatusBanner.test.tsx` (5): role/aria-live matrix across all severities, detail rendering, sr-only severity announcement, dismiss interaction, non-dismissible absence.
- `LoadingButton.test.tsx` (3): idle click-through, pending inert + aria-busy + spinner a11y, disabled independence.
- `check:frontend` green — **25/25** tests.

## Acceptance criteria
- [x] Loading, error, success (and info/warning) feedback components
- [x] Screen-reader semantics per severity
- [x] Reduced-motion respected
