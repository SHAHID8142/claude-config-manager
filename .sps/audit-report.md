# SPS first-contact audit — 2026-11-17

**Baseline alignment: 75/100 — Mixed.** This measures workflow alignment, not
the app's reliability or a completed line-by-line review. Baseline memory and
root workflow files were absent; this audit bootstraps them without changing
application code.

## Scored checklist

| ID | Result | Points | Evidence |
| --- | --- | --- | --- |
| A1 | Fail | 0/3 | No .sps memory at baseline |
| A2 | Fail | 0/3 | No workflow lock at baseline |
| A3 | Fail | 0/3 | No host/version memory at baseline |
| A4 | Fail | 0/2 | No profile/handoff at baseline |
| A5 | Fail | 0/3 | No root mirrors at baseline |
| B1 | Pass | 4/4 | macOS CLI configuration utility, PRODUCT/README/source |
| B2 | Pass | 4/4 | Rust/Tauri + React/TypeScript manifests |
| B3 | Pass | 4/4 | Local utility; no CMS/storefront/ERP/admin service |
| B4 | Warning | 2/4 | Apple Silicon DMG known; clean-Mac/remote CI unverified |
| C1 | Pass | 3/3 | Findings distinguish observations, code traces and uncertainty |
| C2 | Pass | 3/3 | Known/assumed/unverified in plan and review |
| D1 | Warning | 3/6 | Desktop sizes known; actual runtime layout unverified |
| D2 | Warning | 2/4 | Keyboard controls exist; info interaction/target sizes need review |
| D3 | Pass | 6/6 | Minimal CSS motion and reduced-motion path; no scene/video workload |
| D4 | Pass | 3/3 | Desktop-only, minimal motion; no richer mobile path needed |
| E1 | Pass | 5/5 | Asset/build output and source inventory reviewed |
| E2 | Pass | 3/3 | No app autoplay media; frontend payload measured |
| F1 | Pass | 4/4 | Lint/format/typecheck/test/build scripts run successfully |
| F2 | Pass | 3/3 | Runtime, trust and packaging blockers stated |
| F3 | Warning | 1/3 | Structural review done; native/VoiceOver checks unavailable |
| H1 | Pass | 8/8 | CMS explicitly out of scope; local persisted data inventoried |
| H2 | Pass | 8/8 | No storefront/CMS coupling obligation in this desktop utility |
| H3 | Warning | 2/4 | Utility design direction reviewed; real browser finish gate unavailable |
| G1 | Pass | 4/4 | Section-based repair plan written |
| G2 | Pass | 3/3 | Recovery/credentials/switching/UI/release ordered by risk |

## Main finding

The current settings file is **0 bytes**, so it is invalid JSON. The app refuses
to overwrite it but supplies no useful recovery flow. Its status-read error is
also swallowed, which can leave the Dashboard loading indefinitely. Valid
backup candidates exist. The origin of truncation is unverified.

Additional source-confirmed issues include broken legacy aliases on direct
Apply; inconsistent stored-key lookup; helper/credential mutations outside
settings rollback; blocking synchronous commands; stale status; hidden context
resets; unsupported direct-protocol paths; and misleading active-key claims.

The user needs keys tried **within each provider**, then the next provider.
The app currently exposes individual routing targets and does not automatically
turn all stored keys into that sequence.

## Passes and limits

Frontend checks and 201 tests pass. Rust fmt/clippy/tests pass (332 executed,
five live tests ignored). Frontend build passes; npm audit reports zero known
vulnerabilities. The local DMG checksum and installed app codesign verification
pass, but Gatekeeper rejects the ad-hoc-signed app. Installed/local release
executables match. No live inference or clean-Mac acceptance pass is claimed.
The separate optional bridge lock has three moderate affected packages and must
be reviewed before enabling that currently unavailable feature.

## Reviewable remediation

1. Safely recover invalid settings and show truthful status/errors.
2. Repair key resolution, migration and full transaction rollback.
3. Implement provider groups, internal key order and bounded automatic switching.
4. Simplify setup, saved/draft/tested state and add consistent `i` help.
5. Reconcile docs, complete remaining audit coverage and validate a new DMG.

Evidence, 30 findings and 54 unchecked todos:
[engineering review](../docs/audits/2026-11-17-project-review.md).
Concrete implementation/verification scope: [plan](plan.md).
Section and field copy: [help inventory](help-inventory.md).

The original project had no SPS memory. This first-contact audit's required
next step is user choice: approve the repair plan, revise it, or retain audit
only. No implementation or user configuration repair has been performed yet.
