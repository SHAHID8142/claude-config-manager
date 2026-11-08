# T20 — Masked API key entry and reveal/hide interaction

**Date:** 2026-11-08 · **Result:** ✅ PASS (after 2 focused fix cycles, within the 3-cycle budget)

## Implementation
- **`ApiKeyField`** component: masked by default (`type="password"`), explicit Show/Hide toggle with `aria-pressed` state, **auto-rehide after 30 s** (walk-away protection), immediate re-hide on blur, monospace masked styling.
- Security behaviors enforced by the component contract: value lives only in controlled React state; no logging, no clipboard copy, no persistence (SecretStore owns persistence from T21). `autocomplete="off"` + `spellCheck=false`.
- FieldRow hint linkage works through the composite component: `aria-describedby` is forwarded from FieldRow's merge to the inner input (verified by test).
- Wired into App: enabled once a provider (preset or custom) is selected; hint explains Keychain storage.
- Test technique notes: fake timers require synchronous `fireEvent` + explicit `act()` wrapping for state flush (React 19 + RTL); `userEvent` runs on real timers. Documented in the test file.

## Security review (checklist §Secrets)
- [x] Reveal control auto-hides (30 s + on blur)
- [x] No full-key copy paths; no page-text echo of the value (asserted: revealed value never appears outside the input)
- [x] Masked display default

## Tests (evidence)
- 7 component tests: masked default + no echo, toggle/aria-pressed, exact-timeout auto-rehide (29,999 ms → still revealed; +1 ms → hidden), blur re-hide, onChange propagation, disabled synchronization, describedby forwarding.
- App-level: field disabled until provider selected; enabling + typing + reveal flow.
- Frontend gate **44/44** green.

## Acceptance criteria
- [x] Masked API key entry
- [x] Reveal/hide interaction with a11y state
- [x] Auto-rehide security behavior tested
