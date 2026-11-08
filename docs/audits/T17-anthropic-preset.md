# T17 — ONE verified provider preset (Anthropic) with test fixtures

**Date:** 2026-11-08 · **Result:** ✅ PASS

## Implementation
- **Rust:** `src-tauri/presets/anthropic.json` + `src/presets.rs` loader. Presets are `include_str!`-embedded at compile time (no runtime preset file loading → tamper-resistant; threat T7). A builtin failing validation is a test failure by construction.
- Preset facts (verified against official docs, rule 9):
  - protocol `anthropic_native`, bridge `not_required`, auth `api_key` (x-api-key per T02 auth matrix), no base-URL override
  - discovery: `GET https://api.anthropic.com/v1/models`, auth required, `list_selector: "data"` — verified via platform.claude.com/docs/en/api/models-list (redirect chain docs.claude.com → platform.claude.com confirmed live)
  - capability_status: `verified` with source citation
- **TS:** mirror fixture `src/lib/fixtures/providers.ts` (Rust catalog is source of truth; drift guarded by contract wiring in T18) + structural tests.

## Security review
- Compile-time embedding removes runtime preset-tampering risk; fixture contains only public endpoints and doc URLs — no secrets.

## Tests (evidence)
- Rust **17/17** (4 new preset tests: parse/validate, native-direct fields, documented discovery shape, unique ids).
- Frontend **29/29** (4 fixture tests).
- clippy -D warnings clean; fmt clean; frontend gate green.

## Acceptance criteria
- [x] ONE verified preset with official-source citation
- [x] Test fixtures both sides
- [x] Fake/no credentials (preset contains none)
