# Design Decisions — v4 redesign

**Date:** 2026-11-13 · **Companion to:** `UX-AUDIT.md` · Every decision cites its research source and the audit finding it resolves.

## Research basis

1. **NN/g, "8 Design Guidelines for Complex Applications"** (Kaplan, 2020) — progressive disclosure for advanced options; make important information salient, noting *"removing nonessential elements can be equally or even more effective"*; ease transitions between primary and secondary information; flexible nonlinear pathways.
2. **NN/g, "Website Forms Usability: Top 10 Recommendations"** — labels immediately above fields for shorter desktop forms; visually group related label+field; explain input requirements up front; errors need multiple cues, not color alone.
3. **macOS Settings window conventions** (usagimaru, 2025, codifying HIG) — two-column "center equalization" for classic settings rows; headings 13 pt regular, description text 11 pt secondary; ≥20 pt margins around separators; positive phrasing ("Show/Enable…", never "Don't…"); restore the last-displayed panel; every tab carries an icon + label.
4. **Apple HIG — Sidebars** — sidebar navigation suits category-driven settings with growing IA; keep labels short; accent marks the selection.
5. **macOS instant-apply convention — deliberately NOT adopted wholesale.** Classic settings dialogs apply instantly because they mutate app state; this app performs a **transactional write to a live config file** (backup → atomic write → verify → rollback, PRD F07). An explicit Apply is a safety feature, so we keep it — but make the pending ("draft") state explicit at the point of change, and keep the persistent action bar with a per-context hint. (Baymard/Primer auto-apply guidance applies to reversible preference toggles, not to config-file transactions; UX StackExchange consensus: explicit save for risky/grouped changes.)

## Decisions

| # | Decision | Resolves | Source |
|---|---|---|---|
| D1 | **Persist session context**: selected provider id and active tab survive relaunch via `localStorage` (theme already does). | F1 | macOS "restore last-displayed panel"; NN/g "help users track actions" |
| D2 | **One source of truth for selection.** The Dashboard derives its cards from the same selection state as Provider/Credentials/Model; "configured" in the header can never contradict "no provider selected" below it. | F1, F5 | NN/g complex-app #8 (salience via consistency) |
| D3 | **Contextual action bar.** Footer hint is computed per tab: provider-dependent tabs get provider guidance; Failover shows routed/direct state; Maintenance shows "nothing to apply — changes happen on other screens". | F3 | macOS modeless-settings convention (adapted to transactional writes) |
| D4 | **Wider, honest content column.** `max-w-[52rem]` + responsive two-column card grids on Dashboard; panels fill the pane. Sparse tabs get real content (Maintenance gains a backups list backed by a new `list_backups` command). | F2, F9 | NN/g "remove nonessential elements" ≠ remove content; macOS pane usage |
| D5 | **Guided dependent states.** Provider-scoped tabs show a purposeful empty state ("Choose a provider first" + a jump button) instead of zeroed stats or broken headings. | F5 | NN/g dependent-settings + empty-state practice |
| D6 | **Failover regrouped into two cards**: "Runtime" (helper + proxy state merged, one status line, Start/Stop as one toggle-style pair) and "Failover chain" (targets + one primary "Save chain", secondary "Check targets"). One primary button per card; destructive/secondary actions demoted. | F4 | NN/g complex-app #6/#7 (progressive disclosure, clutter) |
| D7 | **Hierarchy pass on tokens.** Section titles: 13 px semibold sentence-case (macOS style, drop the 11 px uppercase micro-label); Dashboard hero card visually promoted (accent keyline, larger type); secondary cards demoted to quiet rows. Light theme: ground tone kept, panel borders strengthened one step. | F6 | macOS Settings conventions (13 pt headings, secondary 11 pt) |
| D8 | **Never truncate silently.** Paths and long notes get `break-all` + a copy button or an expandable disclosure. | F7 | NN/g error/visibility guidance (multiple cues) |
| D9 | **Model tab consolidated**: declared-window explainer moves inside the disclosure next to its input; Draft-vs-Applied stays as the top card. | F8 | NN/g progressive disclosure (context of use) |
| D10 | **Sidebar**: order kept (it matches the real workflow: choose provider → credentials → model → failover → maintain); badge text made explicit ("2 targets", not "on"). | F10 | macOS "every tab needs icon + label"; sidebar HIG |

## Scope guardrails

- No framework change; no new dependencies beyond `lucide-react` usage already present.
- The honest-state policy (never fabricate status; never auto-select) is preserved everywhere.
- Every fix ships with a component or App-level test; screenshots re-taken for the audit trail after implementation.
