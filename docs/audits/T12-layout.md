# T12 — Minimal single-window macOS-friendly layout

**Date:** 2026-11-08 · **Result:** ✅ PASS

## Implementation
- Replaced Tauri template UI with the PRD main-screen shell: header + connection-mode indicator, Active configuration summary, Provider section (selector/endpoint/API key fields), Model section, Actions (Test Connection / Save & Apply), collapsible Advanced.
- Layout primitives: `Section` (labelled region card), `FieldRow` (label + control + hint).
- **Honest-state policy:** controls not yet wired render disabled with explanatory hints ("Providers appear once verified presets ship", "mode: not configured") — no fabricated status (F12/accuracy discipline from T25 applied early).
- Cleanup: template App.css/assets removed; `index.html` title/lang set to the app name; window 800×600 min 640×480 already set in tauri.conf.json (T08).

## Security review
- No IPC calls, no data handling yet. Static shell only; a11y labels present for all controls (screen-reader ready ahead of T14).

## Tests (evidence)
- `App.test.tsx` (4 tests): title, all four PRD sections present as regions, all unwired controls disabled with honest placeholders, mode indicator accessible name.
- Fixes during task: (1) RTL cleanup not running without Vitest globals — explicit `afterEach(cleanup)` in setup; (2) `getByLabelText` ambiguity between section `aria-label` and field label — selector-scoped queries.
- `npm run check:frontend` → lint/format/typecheck ✓, vitest **11/11** ✓; `npm run build` ✓.

## Acceptance criteria
- [x] Minimal single-window layout matches PRD main-screen inventory
- [x] Honest disabled states (no fake data)
- [x] Component tests green
