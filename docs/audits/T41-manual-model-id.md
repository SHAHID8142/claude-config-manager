# T41 — Manual Model ID entry when discovery is unavailable

**Date:** 2026-11-08 · **Result:** ✅ PASS (work resumed from an interrupted session and verified) · **Artifact:** `src/App.tsx` (Model section)

## Context
This task was started by an interrupted session that left `src/App.tsx` and `src/App.test.tsx`
uncommitted. The diff was reviewed against the T41 acceptance criteria and completed rather than
discarded: the entry path was present, the "discovery unavailable" (failure) path was untested, and
the fixtures did not match the real backend error text.

## Implementation
- **Model field is editable whenever a provider (or custom) is selected** —
  `disabled={selected === null && !isCustom}`. Previously it was disabled unless a catalog had been
  fetched, so manual entry was impossible for providers with no documented catalog (Z.AI) and for
  custom gateways (PRD F05).
- **Catalog mode is preserved (T40):** while a populated catalog is displayed the field stays
  `readOnly` and selection remains explicit-click-only; nothing is ever auto-selected.
- **Provider switch resets model state:** `onChange` on the selector clears `selectedModel`,
  `models`, `modelsFetchedFor` and `modelsError` — a model id or catalog from one provider can no
  longer leak into another (stale-catalog defect found during review).
- **Honest copy:** the empty-catalog line ("use manual Model ID entry below") and the field hint
  ("Selected from the catalog or entered manually") now describe behaviour that actually exists.

## Security review
- No new data leaves the renderer: the manual id is local state only. It is **not** written to
  `settings.json` in this task — validation and Save & Apply integration are T43's scope by design
  (backlog: "T43 Add explicit model selection validation and Save & Apply integration").
- No logging, no telemetry, no secret material touched. The field still renders provider-supplied
  ids through React escaping (no `dangerouslySetInnerHTML`).

## Tests (evidence)
`src/App.test.tsx` — new `describe("Manual Model ID entry (T41)")`, 3 cases + 1 negative:
1. editable for a manual-entry provider (Z.AI) and for custom providers, with reset on switch;
2. switching providers resets a previously typed model id;
3. **negative/PRD F05:** after a *discovery failure* the error banner appears and manual entry is
   still enabled, writable and non-readonly;
4. read-only regression guard: with a populated catalog loaded the field stays `readonly`.

Fixture error text corrected to the backend's real `DiscoveryError::InvalidEndpoint` message
("provider template declares no catalog endpoint").

Gates run locally:
- `npm run lint` ✓ · `npm run format:check` ✓ · `npm run typecheck` ✓
- `npm run test` → **82/82 passed** (9 files; was 78/78 at T40)
- `npm run build` ✓ (239.01 kB js / 17.75 kB css)
- `cargo fmt --check` ✓ · `cargo clippy -- -D warnings` ✓ · `cargo test` → **142 passed, 1 ignored
  (live network) + 7 integration = 149** ✓ (Rust unchanged this task; baseline re-confirmed)

## Acceptance criteria
- [x] Manual Model ID entry available when discovery is unavailable/unsupported/failed
- [x] Searchable-catalog path unchanged; nothing auto-selected (T40 invariant held)
- [x] Negative case covered (failed discovery → manual entry still possible)
- [x] No unrelated behaviour changed

## Known limitations (deferred, tracked)
- The manual id is **not yet validated or applied** — T43 owns
  `config_merge::validate_model_id`-equivalent checks and `save_and_apply { model }` wiring.
- The **Fetch models** button is still offered for providers whose template declares no catalog
  (Z.AI), producing a typed error banner rather than a disabled control. Fixing that needs a
  discovery-capability field on `ProviderSummary` (contract change) → T44 scope.
- `cargo clippy --all-targets -- -D warnings` reports 8 pre-existing lints in **test code**
  (`credential_service.rs` bool assertions, `contains` vs `iter().any`, needless `mut`). The
  project/CI gate (`cargo clippy -- -D warnings`, as wired in `package.json` `check:rust` and
  `.github/workflows/ci.yml`) is green; flagged for the T59–T63 hardening sweep.
