# T25 — Explicit capability/compatibility labels

**Date:** 2026-11-08 · **Result:** ✅ PASS (one focused fix, cycle 1 of 3)

## Implementation
- **`CapabilityBadge`**: `capabilities verified` (green, with cited `(source)` link) vs `capabilities unknown` (neutral chip) — "unknown" is a first-class display state, per PRD F12.
- **`BridgeBadge`**: `direct connection` (blue) vs `bridge required (arrives in a later milestone)` (amber, honest availability note — no pretending the bridge exists before T47+).
- **App integration**: badges replace the plain-text notes for every selected preset; the read-only endpoint field now renders only for Anthropic-protocol presets (OpenAI-only providers correctly show no Anthropic base URL).
- One fix cycle: endpoint field initially still rendered for OpenAI-only providers; condition corrected to `protocol !== "open_ai_only"`.

## Security review
- Labels derive exclusively from preset metadata; no hardcoded capability claims in the UI. Unknown renders as unknown — nothing invented.

## Tests (evidence)
- 6 component tests: verified with/without source link, unknown state, direct vs bridge badges.
- App tests: anthropic → verified+direct badges with source link; nvidia-nim → bridge-required badge, no direct badge, **no Anthropic endpoint field**.
- Frontend gate **57/57** green.

## Acceptance criteria
- [x] Verified / unknown / bridge-required labels explicit and data-driven
- [x] No fabricated capability values
