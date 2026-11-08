# T27 — Parse and preserve unrelated settings, with invalid JSON tests

**Date:** 2026-11-08 · **Result:** ✅ PASS (one focused fix, cycle 1 of 3)

## Implementation
- `config_reader::read_settings(path)` — strict JSON per T02 matrix (serde_json rejects comments/trailing commas natively); malformed input yields `MalformedJson("line N column M")` with **no content leakage into errors** (asserted).
- **Never overwrite invalid source**: malformed → error; documented contract that nothing downstream may write until resolved.
- State handling: `Missing` (first-run), `SymlinkRefused` (threat T5, via T26 locator), non-object roots rejected with a shape name (`an array`, `a string`, …), `PermissionDenied` mapped distinctly.
- **Preservation mechanics**: `serde_json` with `preserve_order` feature — key insertion order survives round-trip (diff-friendly, T02 requirement); `unrelated_keys(doc, managed)` enumerates keys outside the app's managed set so T28's merge can prove F02 preservation.

## Security review
- Error messages carry positions only, never file content (secret leakage guard, threat T2).
- Symlink refusal inherited from the locator (no TOCTOU read-through).

## Tests (evidence)
- 9 tests: valid parse, missing, symlink refusal, **trailing comma** (position info, no content leak), **comments rejected**, non-object root, unrelated-key round-trip (`statusLine`/`permissions` survive), **key-order preservation**, permission-denied (chmod 000, restored after).
- `cargo test` **69/69**; clippy -D warnings clean; fmt clean.

## Acceptance criteria
- [x] Parse preserves unrelated settings and order
- [x] Invalid JSON cases covered (malformed, comments, wrong shape)
- [x] Never-overwrite-invalid-source contract established
