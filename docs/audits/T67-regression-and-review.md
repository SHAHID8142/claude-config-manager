# T67 — Complete regression run and independent review of acceptance criteria

**Date:** 2026-11-08 · **Result:** ✅ PASS for everything claimed; **5 tasks remain unproven and are
listed as such** (T47–T58, T65, T69–T70 status per `TASK_PROGRESS.md`)

This is a review of *claims*, not a summary of them. For each block the acceptance criteria were re-checked
against committed code and a fresh test run, not against the earlier audit prose.

## 1. Fresh full-suite results (this commit)
```
cargo fmt --check                         PASS
cargo clippy --all-targets -- -D warnings PASS (0 warnings)
cargo test                                201 passed, 0 failed, 2 ignored (live network)
npm run lint                              PASS
npm run format:check                      PASS
npx tsc --noEmit                          PASS
npx vitest run                            120 passed / 12 files
npm run build                             PASS
scripts/dep-audit.sh                      PASS (0 advisories in the macOS graph)
```
Breakdown of the 201 Rust tests: 156 unit (+2 `#[ignore]` live-network) · 7 `adversarial_paths` ·
7 `crash_recovery` · 6 `lifecycle` · 14 `malformed_inputs` · 11 `secret_and_network_surface`.

## 2. Do the audits cite real tests?
Automated check across all 54 audit documents: **53 backticked test identifiers extracted, 0 not found in
the codebase.** Every named regression test exists. (Three identifiers were too generic to check and were
excluded by the length filter — no false PASS is claimed for them.)

## 3. Do the cited tests actually pass?
The full run above lists every test by name; all cited names appear as `... ok`. No cited test is skipped,
and the only `ignored` tests are the two deliberate live-network ones.

## 4. Acceptance criteria re-checked by block
| Block | Claim | Independent check | Verdict |
|---|---|---|---|
| T01–T07 discovery/risk | provider matrix, auth precedence, threat model | `docs/audits/T0*`, tests present & green | ✅ |
| T08–T15 foundation | IPC contract, error shape, a11y primitives | `App.test.tsx` + component tests green | ✅ |
| T16–T25 provider/credential UX | Keychain storage, redaction, helper mode | `credential_service`/`keychain` tests + real-Keychain test (T22) re-run in sweep | ✅ |
| T26–T36 config safety | merge/backup/atomic write/conflict/restore | 7 crash-recovery + 7 adversarial tests green | ✅ |
| T37–T42 model discovery | catalog adapters, provenance, manual entry | OpenRouter & NVIDIA live-verified; DeepSeek docs-verified (T44b) | ✅ with limits |
| T43–T46 selection/apply | model written & validated, roles default-off, no failover | `App.test.tsx` (T43/T45 blocks) + `save_apply` guards | ✅ |
| T59 malformed inputs | 14-case suite | green; 5 defects fixed | ✅ |
| T60 network/secrets | 11-case suite | green; 5 defects fixed | ✅ |
| T61 crash recovery | 7-case suite | green | ✅ |
| T62 a11y/theme/small window | 10-case suite | green; real macOS pass **NOT TESTED** | ✅/⚠️ |
| T63 dependencies | 0 advisories in macOS graph | gate re-run now | ✅ (license blocker open) |
| T64 package | unsigned arm64 build | artifacts exist, Gatekeeper **refuses** | ⚠️ blocked |
| T66 lifecycle | helper/closure/uninstall/restore | 6-case suite green | ✅ |

## 5. Claims that are NOT verified (stated plainly)
- **T47–T58 (bridge)**: not started. No bridge feature exists; nothing is claimed.
- **T65 (clean-install smoke test)**: no evidence. Not performed.
- **T10 CI**: the workflow file exists but has **never executed** (GitHub billing lock). Local gates are the
  only evidence.
- **Real macOS GUI**: no launch, no VoiceOver, no light/dark toggle, no window resizing. The unsigned build
  cannot be launched normally (Gatekeeper), so this cannot be improvised here.
- **Live model discovery for DeepSeek/Anthropic**: requires a real API key (rule 15) — docs-verified only.
- **Project license**: absent; owner decision.

## 6. Regressions introduced during the hardening block?
The hardening tasks (T59–T63) changed validation behaviour. Re-checked that no preset, fixture or documented
flow broke:
- all 5 bundled presets still parse and validate (`provider_catalog` tests),
- `save_and_apply` still accepts every documented input shape (`save_apply` tests),
- the UI gate still enables apply for a valid provider+key (`App.test.tsx`),
- `classify_endpoint` change (non-http → `Invalid`) updated its own unit test rather than being silently
  dropped.

## 7. Verdict
Every ✅ in `TASK_PROGRESS.md` is backed by a test that exists and passes in this run, except where the row
itself already says otherwise (T10 CI, T64 package). Nothing in this repository is marked ✅ on the strength
of an unverifiable claim.
