# V3-29 / V3-30 — Window, layout and accessibility verification

## Window behaviour (real app, real screen)

Build under test: `src-tauri/target/release/bundle/macos/ Claude Config Manager.app`
(ad-hoc signed), installed to `/Applications` and launched.

| Size | What was checked | Result |
| --- | --- | --- |
| Minimum `620 × 560` (the window's declared minimum) | text clipping, overlap, footer reachability, sidebar reachability | no clipped or overlapping text; footer's reason sentence and button both visible; the *Appearance* segmented control sits fully inside the sidebar without scrolling; the content area scrolls, which is intended |
| Normal `760 × 780` | header pill text, sidebar items, single primary action, truncation | pill reads `mode: configured`; sidebar shows Dashboard (selected), Provider, Credentials, Model, Failover, Maintenance, then System/Light/Dark; footer shows one primary action plus "Choose a provider to enable Save & Apply."; long values (`nvidia/nemotr…`, `https://openro…`, `1,000,000 tok…`) truncate rather than wrap or clip |

Screenshots: `.taste-skill/v3-min.png`, `.taste-skill/v3-normal.png` (git-ignored;
captured with `screencapture -R` over the resolved window rect). They were read
through a vision model, which is why the wording above is a description of what
it reported plus what the DOM tests assert — **not** a claim of pixel-exact
measurement.

Resizing is handled by one scrolling column inside a fixed sidebar and a fixed
footer, so there is no horizontal scrollbar at the minimum size and the footer
never leaves the screen.

## Accessibility (code-level suite, green)

`src/a11y.test.tsx` plus the per-component suites enforce:

- exactly one `h1`, no skipped heading levels;
- no positive `tabIndex`, every interactive control reachable and focusable;
- every form control has a programmatic name (`label`, `aria-label` or
  `aria-labelledby`);
- decorative glyphs and badges are `aria-hidden`, so "Credentials" is announced
  without its count;
- no fixed `w-[Npx]` widths; every `min-w-0` box also truncates or breaks words;
- the sidebar marks the current screen with `aria-current="page"`;
- status is exposed as `role="status"` text, not colour alone (the dot is
  `aria-hidden` and the sentence carries the meaning);
- destructive actions (remove a credential, restore a backup, switch away from
  routed mode) require an explicit second action;
- loading and error states are announced (banners are in the accessible tree, not
  hover-only).

`npm run lint`, `npx tsc --noEmit` and `vitest run` (196 tests / 25 files) pass.

## Not verified — stated explicitly

- **No VoiceOver session was run.** Screen-reader *labels* are asserted in jsdom;
  the spoken experience on a real Mac is unverified.
- **No contrast measurement tool was run.** Colours come from the token table and
  were chosen against WCAG AA expectations, but no automated contrast audit was
  executed in this phase.
- **No true 1× display or external-monitor test**; screenshots were taken on this
  Mac only.
- **Keyboard-only traversal of every screen** was not performed end to end; focus
  order is asserted structurally in tests.
