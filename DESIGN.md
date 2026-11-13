# DESIGN.md — Claude Config Manager

**Mode: Operate.** The visitor completes a task; scanability, consistency, and native macOS expectations outrank expression. The brand lives in precise details. (Impeccable v4.7 conventions; decisions traceable to `docs/design/DESIGN-DECISIONS.md`.)

## World

"The instrument panel for a warm-toned CLI." The world contributes exactly four things:

1. **Type** — the platform stack (`system-ui`, SF Pro on macOS; the platform font is the native expectation for a macOS utility, not a display-face shortcut). Scale: 19/16 semibold page+card titles, 13 base, 12 secondary, 11 micro. Tabular numerals for all metrics. Monospace only for identifiers (model ids, paths, code) — never as a "technical" costume.
2. **Palette** — warm neutral ground (`--app-bg`), white surfaces, graphite ink, **one** accent (clay `#b85731`) used for selection, the hero keyline, and the primary-action focus; functional colors (ok/warn/danger) only where they carry meaning. Light and dark are equal citizens, driven by `data-theme`.
3. **Density** — comfortable-operate: 8 pt control rhythm, 20 pt group separation, cards 12–14 px radius, content column ≤52 rem, zero dead columns.
4. **Signature move** — **the apply transaction**: the one authored motion moment. When Save & Apply runs, the action button shows the sequence (backup → write → verify) as a single compact progress state, and on success the hero card's keyline sweeps once (clip-path, 600 ms, ease-out) — the app's way of saying "the file changed underneath you, and it's safe". Everywhere else motion is feedback-only: 120–180 ms ease-out on hover/press, no entrance choreography, `prefers-reduced-motion` collapses everything to instant.

## Structure

- Sidebar–detail shell (macOS System Settings pattern): 6 screens, icons + labels, current screen marked by surface pill + accent icon.
- **Row pattern**: section = label (13 px semibold) + one-line description + control panel. Long explanations live in `HelpPopover` (secondary info without leaving the primary view), never as paragraphs on the working screen.
- The **persistent action bar** carries the one primary action per screen with a context-specific hint (never a generic "choose a provider" on screens that don't need one).
- Dependent screens show **guided states** (explain + jump button), never zeroed stats.

## Bans (project-adopted, from the craft floor)

No identical-card grids as page structure; no eyebrow/kicker labels; no gradient text; no glow halos; no side-stripe borders above 1 px except the hero keyline (which is the signature move, 3 px, on exactly one card); no glass/blur decoration; no monospace-as-costume; no emoji icons. Elevation declared once per card (border *or* shadow, never both).

## Browser surfaces (shipped, not defaulted)

`::selection` in accent-soft, caret in ink, styled scrollbars (thin, neutral), focus ring = `--ring` everywhere, `font-variant-numeric: tabular-nums` on all numeric data, underline offset on links.
