# T18 — Provider selector + custom provider entry

**Date:** 2026-11-08 · **Result:** ✅ PASS

## Implementation
- **Rust:** new `list_providers` command returning `ProviderSummary` (camelCase wire shape) built from builtin templates; registered in the invoke handler; wire-shape contract test asserts camelCase keys.
- **TS:** typed `ProviderSummary` + `listProviders()` in the contract module; `providerSummaryFixtures` mirror added for mocks.
- **App:** provider selector loads the catalog over IPC (injected `IpcLike`, production `defaultIpc`); displays verified-capability source link and direct-mode note for presets; read-only preset endpoint (native Anthropic shown); **Custom…** entry reveals editable custom name + endpoint inputs (validation lands in T19). Honest placeholders preserved for unwired controls (API key, model, actions).
- T12/T14 shell + a11y regression tests restored alongside the new selector tests.

## Security review
- Selector consumes only compile-time-embedded verified templates; no user data crosses IPC yet; custom entry captures text but applies nothing (apply arrives T33 with validation).
- Error path: catalog load failure surfaces a StatusBanner (role=alert), not a silent blank state.

## Tests (evidence)
- Rust **18/18** (new: list_providers returns bundled templates + camelCase wire shape).
- Frontend **29/29**: catalog load + option list; preset selection shows verified/direct notes + read-only endpoint; custom entry typing; catalog failure banner; shell/a11y regressions.
- Full gates green both toolchains.

## Acceptance criteria
- [x] Provider selector wired to verified presets
- [x] Custom provider entry (name/endpoint captured; validation in T19)
- [x] No fabricated status; error state handled
