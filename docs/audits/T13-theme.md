# T13 — Light/dark system theme + responsive sizing

**Date:** 2026-11-08 · **Result:** ✅ PASS

## Implementation
- `src/index.css`: `:root { color-scheme: light dark; }` (native controls/scrollbars follow OS appearance) + system font stack (system-ui/-apple-system — macOS-native feel).
- Dark styling: Tailwind v4's `dark:` variant defaults to the `prefers-color-scheme` media query → the UI follows the system theme with **no runtime toggle and no flash**; both palettes already present on every element from T12.
- Responsive sizing: window bounds 800×600 (min 640×480) set in `tauri.conf.json`; layout is a single-column flex stack with `max-w-2xl` and wrapping action row → degrades gracefully at the 640px minimum.

## Verification (evidence)
- Built CSS contains `prefers-color-scheme` blocks (Tailwind dark variant = media-query based, i.e. system-driven) and the `color-scheme` rule — confirmed by grepping `dist/assets/*.css` after `npm run build`.
- jsdom cannot evaluate media queries; this is the strongest automatable evidence. Manual visual check on real macOS deferred to T62/T65 (theme audit), matching TEST-STRATEGY layering.

## Security review
- CSS-only change; no data, no IPC.

## Tests
- Full frontend gate re-run green (lint/format/typecheck/11 tests).

## Acceptance criteria
- [x] Light/dark system theme (media-query driven, verified in build output)
- [x] Responsive sizing bounds + flexible layout
