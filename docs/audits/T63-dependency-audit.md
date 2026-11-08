# T63 — Dependency, license and vulnerability audit

**Date:** 2026-11-08 · **Result:** ✅ PASS for the shipped macOS graph · **1 release blocker: no license**
**Artifact:** `scripts/dep-audit.sh` (re-runnable gate, exit 1 on any shipped-graph advisory)

## Method
Version-aware, not name-based. All **resolved** versions are inventoried (npm from `node_modules`,
cargo from `Cargo.lock` — 750 unique packages), queried against the OSV advisory database, then filtered
to what actually reaches a macOS build (`cargo tree --target aarch64-apple-darwin`; every npm package
counts). `npm audit` runs registry-side as a second, independent check.

## Vulnerabilities
```
npm audit --audit-level=high      → found 0 vulnerabilities
resolved packages checked         → 750
affecting the macOS/shipped graph → 0
not in the macOS graph            → 6
```
The six exclusions are Linux-only GTK/D-Bus paths and a build-time proc-macro helper that the
`aarch64-apple-darwin` dependency graph does not contain (verified: `cargo tree -i glib` is empty):

| Crate | Version | Advisory | Why excluded |
|---|---|---|---|
| glib | 0.18.5 | GHSA-wrw7-89jp-8q8g / RUSTSEC-2024-0429 | Linux GTK stack only |
| proc-macro-error | 1.0.4 | RUSTSEC-2024-0370 (unmaintained) | build-time only, not in graph |
| zbus_names | 4.3.5 | RUSTSEC-2026-0371 | Linux D-Bus only |
| zvariant | 5.15.1 | RUSTSEC-2026-0370 | Linux D-Bus only |
| zvariant_derive | 5.15.1 | RUSTSEC-2026-0372 | Linux D-Bus only |

They are **not remediated** (no version bump attempted): patching Linux-only transitive pins of Tauri
would risk the macOS build for zero shipped benefit. They stay visible in the gate output and will be
re-evaluated when Tauri's own pins move — or immediately if a Windows/Linux port (T68 notes) is started.

## Licenses
| Ecosystem | Result |
|---|---|
| npm (269 packages) | MIT 206, Apache-2.0 20, ISC 15, BSD-2 8, BlueOak 3, Apache/MIT 3, BSD-3 3, MPL-2.0 2, 0BSD 1, Python-2.0 1, CC-BY-4.0 1 — **0 GPL/AGPL** |
| cargo (494 packages) | Overwhelmingly `MIT OR Apache-2.0`; 5 `MPL-2.0` (file-level copyleft — `cssparser`, `cssparser-macros`, `dtoa-short`, `option-ext`, `selectors`); 2 triple-licensed including `LGPL-2.1-or-later` as one *option* (MIT/Apache alternatives exist) — **0 GPL/AGPL-only** |

All licenses are compatible with distributing a proprietary or MIT-licensed macOS app; MPL-2.0 requires
only that modifications to those specific files stay under MPL.

## Release blocker found
**The project itself has no license.** There is no `LICENSE` file, no `license` field in `package.json`,
and none in `src-tauri/Cargo.toml` (`cargo metadata` reports the crate as `unknown`). Without a license
the artifact is "all rights reserved" by default, which conflicts with the open-source intent in the PRD
and blocks any redistribution decision. **Choosing a license is a legal/product decision for the owner —
not something this agent will pick.** Flagged in `TASK_PROGRESS.md` as a release blocker for T69.

## Tests / how to re-run
```bash
scripts/dep-audit.sh            # PASS/FAIL for the macOS graph, lists excluded advisories
npm audit --audit-level=high    # 0 vulnerabilities
```
Gate is deterministic except for the advisory database (network); it fails closed on any *shipped* hit.

## Acceptance criteria
- [x] Full dependency inventory (npm + cargo) at resolved versions
- [x] Vulnerability check that distinguishes shipped from non-shipped crates
- [x] License inventory with copyleft analysis
- [x] Critical findings: none in the shipped graph
- [ ] Missing project license — **BLOCKED on owner decision** (not a code fix; recorded for T69)

## Known limitations
- Advisories are only as fresh as the OSV database at run time; the gate must be run in CI/release, not
  assumed from this document.
- `cargo audit` itself is not installed locally (its Linux CI job covers it); the script substitutes a
  version-aware OSV query that is stricter about scope (per-target graph).
- License inventory is metadata-based (SPDX fields); it does not audit vendored files or bundled
  third-party notices that a distribution would need to include.
