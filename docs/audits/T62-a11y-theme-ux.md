# T62 — Keyboard, accessibility, macOS theme and small-window UX audit

**Date:** 2026-11-08 · **Result:** ✅ PASS (jsdom + static evidence; real macOS visual pass **NOT TESTED**) ·
**Artifact:** `src/a11y.test.tsx` (10 checks) + small-window fixes

## What was audited
| Dimension | Method | Result |
|---|---|---|
| Heading structure | jsdom: one `h1`, no skipped levels | ✅ |
| Accessible names | jsdom: every textbox/combobox/button has a name (native `<label for>` counts) | ✅ |
| Focus order | jsdom: all key controls focusable, DOM order = visual order (provider → key → fetch → model → apply) | ✅ |
| Tabindex | jsdom: no `tabindex > 0` anywhere | ✅ |
| Announcements | jsdom: errors `role="alert"`/assertive, others `role="status"`/polite | ✅ |
| Decoration | jsdom: disclosure glyphs `aria-hidden="true"` | ✅ |
| Disclosure semantics | real `<button aria-expanded aria-controls>` for the advanced panel (T45) | ✅ |
| Dark theme coverage | static: every file using a light text colour also ships `dark:` variants | ✅ |
| Native control appearance | `html { color-scheme: light dark }` present in `src/index.css` (verified by inspection — not automatable in jsdom) | ✅ |
| Small window | static: no fixed `w-[Npx]`; long paths/ids/URLs wrap | ✅ (after fix) |
| Reduced motion | static: any `animate-`/`transition-` utility must carry `motion-safe`/`motion-reduce` | ✅ (no motion shipped) |

## Fixes made
1. **Horizontal overflow risk removed**: model ids, base URLs and helper paths are rendered in `<code>`
   without wrapping, so a long `/Users/…/Library/Application Support/…/helpers/deepseek.sh` path or a
   long model id could push the layout wider than a small window. All such `<code>` elements now carry
   `break-all`, and the test enforces it for every new one.
2. Test-side corrections (recorded so the audit is not flattering): my first accessible-name check ignored
   native `<label for>` association (false failure), and my focus-order chain assumed model-before-fetch
   while the UI deliberately fetches first, then lets you choose.

## Honest gaps (why this is not a full a11y certification)
- **No real macOS run**: no VoiceOver pass, no actual window resizing, no System Settings light/dark
  toggling. jsdom has no layout engine, so "small window" is verified at the class level only.
- **Contrast not measured**: colours come from Tailwind's zinc palette with paired `dark:` variants, but
  WCAG contrast ratios were not computed against rendered pixels. Static class pairing is not proof of
  4.5:1.
- **No automated axe/WCAG rule engine** is wired in (no new dependency was added); the checks above are
  targeted assertions, not a full rule sweep.
- Focus-visible styling relies on `focus-visible:ring-*` utilities; verified present in source, not
  observed on screen.

## Tests (evidence)
- `npx vitest run` → **120 passed / 12 files** (includes the 10 new a11y/theme checks)
- `npm run lint` ✓ · `npx tsc --noEmit` ✓ · `npm run format:check` ✓ · `npm run build` ✓

## Acceptance criteria
- [x] Keyboard reachability and focus order asserted
- [x] Accessibility semantics asserted (names, roles, live regions, ARIA pairing)
- [x] macOS theme support verified (dark variants + system colour-scheme)
- [x] Small-window resilience addressed (wrapping, no fixed widths)
- [ ] Real macOS visual/VoiceOver verification — **NOT TESTED** (no display access in this environment;
      belongs to T65 with user evidence)
