# V2-55 — Sidebar navigation and the per-screen information architecture

**Status:** done · **Owner request:** left-side tab system like a native macOS app.

## Why

The single scrolling column made every concern compete for the same eye: a
provider, a key, a model id, a failover chain and a restore button all looked
equally important, and the useful question — *what is Claude Code using right
now?* — was answered by a strip that scrolled away.

## Shape (Apple HIG: sidebar–detail)

```
┌──────────────────────────────────────────────────────────┐
│ app icon · Claude Config Manager            mode: …      │  title row (drag region)
├────────────┬─────────────────────────────────────────────┤
│ Dashboard  │ Dashboard                                   │
│ Provider   │ What Claude Code is using right now, and    │  page heading (h2)
│ Credentials│ what needs attention.                       │
│ Model      │                                             │
│ Failover   │ ┌ Active configuration ──────────────────┐  │
│ Maintenance│ │ mode · model · endpoint · key · window │  │
│            │ └────────────────────────────────────────┘  │
│ ────────── │ ┌ Provider ┐ ┌ Model ┐ ┌ Credentials ┐ …   │
│ Appearance │ └──────────┘ └───────┘ └─────────────┘     │
│ System│Light│Dark                                          │
├────────────┴─────────────────────────────────────────────┤
│ reason Save & Apply is disabled              [Save & Apply]│
└──────────────────────────────────────────────────────────┘
```

- **Sidebar**: 188 px, one flat list, 32 px rows, 6 px radius. The current item
  is a white pill with a 1 px ring; only its Lucide glyph takes the accent. Counts
  (`Credentials 3`, `Failover on`) are `aria-hidden` decorations, so a screen
  reader hears the section name only.
- **Appearance**: a macOS segmented control at the foot of the sidebar
  (`System · Light · Dark`) — one click, no popover, follows the OS live.
- **Content**: one screen at a time, 736 px measure, 8 pt spacing, the primary
  action pinned in a footer with the reason it is disabled.

## What each screen answers

| Screen | Question it answers | Contents |
| --- | --- | --- |
| Dashboard | What is in effect, and what is wrong? | Active-configuration card (mode, model, endpoint, key source, window, warnings), then Provider / Model / Credentials / Failover / Environment cards, each with the action that opens its screen |
| Provider | Where do requests go, and what authenticates them? | Verified preset or custom endpoint, capability + bridge badges, API key field, **Test Connection** |
| Credentials | How many keys exist, and which one is used? | Counts (stored / enabled / handed over), the ordered list with enable, promote, remove, and the add form; where the files live |
| Model | Which model is selected, which can I select, what does Claude Code assume? | Applied vs draft side by side, catalog fetch + list, manual id, validation, declared context window, role overrides |
| Failover | What happens when a target fails? | The ordered chain, cooldown/health badges, proxy start/stop, routed apply, and a plain list of what the proxy will and will not do |
| Maintenance | Environment and rollback. | CLI version, settings state, config-dir writability, shell exports, restore-last-backup |

## Rules the screens keep

- Sentinel ids (`__none__`, `__custom__`) never reach the backend; the
  first-launch "invalid provider id" banner cannot come back.
- An error that happened while saving is reported on the Dashboard, where the
  answer lives — not under the button that caused it.
- Nothing is fetched or reset when a screen is merely opened; discovery and
  connection tests stay explicit.
- Every interactive element has a visible focus ring, a real disabled state and
  a name that does not depend on a count rendered beside it.

## Verification

- `tsc --noEmit` clean; `eslint .` clean.
- Vitest **185 passed / 24 files**, including new `AppSidebar` (3) and
  `ThemeSegmented` (2) suites, and the shell suites rewritten to traverse the
  sidebar (`goTo(...)`) instead of assuming one page.
- `a11y` invariants held: one `h1`, no skipped heading level, no positive
  tabindex, every key control reachable and focusable, no fixed `w-[Npx]`
  widths, long values truncated inside `min-w-0`.
- `npm run tauri build` exit 0; bundle installed and launched, screenshot
  review of the real window confirmed the sidebar, segmented appearance control,
  dashboard cards and footer hint.

**Not verified:** keyboard-only traversal of the whole app on a real Mac with
VoiceOver, and window behaviour at 800×600 (jsdom cannot measure layout).
