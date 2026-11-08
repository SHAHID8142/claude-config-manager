# T42 — Model metadata with source/unknown labels; no invented capability values

**Date:** 2026-11-08 · **Result:** ✅ PASS · **Artifacts:** `model_discovery.rs`, `ipc.ts`, `ModelList.tsx`, `App.tsx`

## Implementation
- **Provenance on the wire:** `ModelEntry` gained `source: String` — the **verified template catalog URL**
  the record was read from. It is taken from the compile-time embedded preset (`DiscoverySpec::Endpoint`),
  never from the response body, so a hostile or compromised catalog cannot claim its own source.
- **Unknown stays unknown:** `capabilityStatus` remains the literal `"unknown"` for every discovered
  entry, and the entry shape is fixed at four fields (`id`, `displayName`, `capabilityStatus`, `source`).
  No context-window / vision / reasoning / tool fields exist to be populated later by accident (PRD F12).
- **UI — catalog path:** `ModelList` renders a provenance line
  `Metadata source: <url> · capability data unverified (unknown for every model)` above the per-model
  `capabilities unknown` chips. An empty/absent source renders `unknown` rather than a guess.
- **UI — manual path:** a typed model id shows `Source: manual entry · capability data unverified (unknown)`,
  and that label disappears as soon as a catalog is displayed (mutually exclusive states).
- **No capability inference:** nothing derives capability from the display name, the id, or the host.

## Security review
- Provenance is **not attacker-controlled**: `source_is_template_url_not_payload_claim` feeds a catalog
  whose body contains `"source":"https://evil.example.com"` and `"context_window":999999`, then asserts
  the reported source is the verified loopback template URL and that no extra fields exist on the wire.
- The `source` URL is a public catalog endpoint from a validated `https` preset (T16 gate); it carries no
  credential and no query string, so displaying it leaks nothing.
- Scope guard: `source` is display-only. It is never written to `settings.json` (only `id` is, in T43).

## Tests (evidence)
Rust (`model_discovery`):
- extended `parses_mock_openai_shape_with_entry_filtering`: every entry's `source` == template URL and
  `capability_status == "unknown"`;
- new `source_is_template_url_not_payload_claim` (hostile-payload negative + fixed wire-shape assertion).

Frontend:
- `ModelList.test.tsx` → new `Model metadata provenance (T42)` (2 cases: source line + no-invented-values
  regex guard; missing source → `unknown`);
- `App.test.tsx` → new `Model metadata provenance (T42)` (3 cases: manual label, catalog provenance with
  manual label absent, no invented capability words in the manual path).

Gates (local, this commit):
- `npm run lint` ✓ · `npm run format:check` ✓ · `npm run typecheck` ✓ · `npm run test` → **87/87 passed**
- `npm run build` ✓
- `cargo fmt --check` ✓ · `cargo clippy -- -D warnings` ✓ · `cargo test` → **143 passed, 1 ignored + 7
  integration = 150** ✓

## Acceptance criteria
- [x] Model metadata carries an explicit, verifiable source label
- [x] Every capability value is displayed as `unknown` / unverified — none invented
- [x] Manual entry is labelled as manual, not as catalog data
- [x] Negative case: hostile payload cannot set provenance or smuggle capability fields

## Known limitations
- `source` is per-catalog, not per-model: one fetch yields one source, and mixed-provenance catalogs are
  not represented. Acceptable while discovery uses one documented endpoint per template.
- Capability data is genuinely absent (not "unknown-soon"): populating it requires per-model verified
  sources → T46 ("only verified Claude Code model-role mapping if needed").
- The manual-entry label is a plain paragraph; if the Advanced panel (T45) later holds provenance, this
  line should move there rather than duplicate.
