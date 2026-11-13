# UX Audit — Claude Config Manager (v4 redesign basis)

**Date:** 2026-11-13 · **Method:** rendered-app inspection via dev harness (`?mock=1`) with screenshots of all six tabs in dark and light themes at 1280×860, plus code-level state analysis (`src/App.tsx`, tabs, tokens). Research basis in `DESIGN-DECISIONS.md`.

## Findings (each traced to evidence)

| # | Severity | Finding | Evidence |
|---|---|---|---|
| F1 | HIGH | **Amnesiac provider state.** `selectedId` resets to "none" every launch (`App.tsx` `useState(EMPTY_ID)`, no persistence). Result: the Dashboard simultaneously claims "mode: configured" + shows the live model, and a Provider card reading "No provider selected — pick a provider to write settings". Credentials tab shows "Keys for no provider". The app contradicts itself at first glance. | Dashboard + Credentials screenshots; `App.tsx:83` |
| F2 | HIGH | **Dead space.** Content column capped at `max-w-[46rem]` (~736 px) in a 1280 px window; Provider/Model/Maintenance tabs are half empty; Maintenance uses ~25 % of the pane. | all screenshots |
| F3 | HIGH | **Context-free action bar.** The footer hint "Choose a provider to enable Save & Apply." renders on Failover and Maintenance too, where it is irrelevant (Failover has its own Apply action; Maintenance has none). The primary button is disabled everywhere with the same message. | Failover/Maintenance screenshots; `App.tsx:525` |
| F4 | MED | **Failover button soup.** Nine similar bordered buttons in three stacked groups (Start helper/Stop helper/Restart; Check targets/Save chain; Start proxy/Stop/Route Claude Code). No clear primary; group boundaries unclear. | Failover screenshot |
| F5 | MED | **Dependent-state leak on Credentials.** Without a provider the tab renders a zeroed stats row ("0 keys / 0 eligible / nothing yet") and a "Keys for no provider" heading instead of guiding the user to pick one. | Credentials screenshot |
| F6 | MED | **Weak visual hierarchy.** Section titles are 11 px uppercase micro-labels; every card is the same weight; the Dashboard's most important fact (what Claude Code is using *right now*) competes with five equal cards. Light theme reads as beige-on-beige with faint structure. | Dashboard light screenshot |
| F7 | LOW | **Truncated content.** "Where keys live" path cuts mid-word ("…sec"); "About the shell" note cuts mid-sentence with no affordance to read more. | Credentials/Maintenance screenshots |
| F8 | LOW | **Model tab fragmentation.** The declared-window explainer floats as a long left-rail paragraph far from the control it explains; "Choose" and the Advanced accordion feel disjointed. | Model screenshot |
| F9 | LOW | **Maintenance is one action on a whole page.** No backups list, no path disclosure, nothing else to justify the screen. | Maintenance screenshot |
| F10 | LOW | **Tab order vs. workflow.** Credentials sits before Model, but the dominant flow is Provider → Model → Apply; credentials are a sub-concern of Provider. Order is defensible but the sidebar badge ("on" on Failover) is cryptic. | sidebar screenshots |

## What is already right (keep)

- Sidebar–detail shell matches macOS System Settings; one screen per concern (NN/g complex-app guidance).
- Semantic token system (warm neutrals, single clay accent, functional colors) is well-built — the redesign reuses it.
- Draft-vs-applied comparison on Model, honest-state policy everywhere, apiKeyHelper education copy.
- Focus visibility, reduced-motion support, radiogroup theme control — good a11y bones.

## Verdict

The bones are good; the failures are **state coherence** (F1, F3, F5), **spatial economy** (F2), and **hierarchy** (F4, F6). All are fixable without a framework change. The redesign in `DESIGN-DECISIONS.md` addresses each finding with a cited principle, and every fix ships with a regression test.
