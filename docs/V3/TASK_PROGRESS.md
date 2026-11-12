# V3 — Stability, Compatibility & Final Polish — task progress

Baseline for V3: `9d17575` on `main` (remote identical), working tree clean,
Final: Rust **314** passed / 0 failed, TypeScript **196** passed / 25 files.

**Status key:** `done` = code + tests + commit pushed · `partial` = implemented but
one verification step is impossible on this machine (stated) · `blocked` = needs
an owner decision · `not verified` = no claim made.

| Task | Title | Status | Evidence |
| --- | --- | --- | --- |
| V3-01 | Repository baseline | done | `docs/V3/V3-01-baseline.md` |
| V3-02 | Verify known defects | done | `docs/V3/V3-02-defects.md` — 6 confirmed, 1 reclassified |
| V3-03 | Regression baseline for confirmed defects | done | `src-tauri/tests/v3_regressions.rs`, `src/v3-regressions.test.tsx` |
| V3-04 | Helper lifecycle design | done | `docs/V3/V3-04-helper-lifecycle.md` |
| V3-05 | Separate proxy from GUI | done | `src-tauri/src/helper.rs`, `main.rs` `--proxy-helper` branch |
| V3-06 | Secure helper startup | done | loopback bind, 0600 token/state, no shell interpolation |
| V3-07 | Lifecycle management (start/stop/restart/health/recovery) | done | helper_status/proxy_start/proxy_stop/proxy_restart commands |
| V3-08 | Persist routed configuration | done | `proxy-helper.port` + persistent token |
| V3-09 | Restart/reboot behaviour + explicit login start | done | `helper_login_start`, opt-in checkbox |
| V3-10 | Lifecycle regression tests | done | `src-tauri/tests/helper_lifecycle.rs` 5 tests |
| V3-11 | Compatibility matrix | done | `docs/V3/V3-11-compatibility-matrix.md` |
| V3-12 | Evaluate open-source bridges | blocked | `docs/V3/V3-12-bridge-evaluation.md` — needs owner choice; licence not verifiable offline |
| V3-13 | Adapter interface | done | `proxy::Adapter` + `adapter_for()` |
| V3-14 | Request translation | blocked | follows V3-12 decision |
| V3-15 | Response translation | blocked | follows V3-12 decision |
| V3-16 | Capability protection | done | `501 compatibility_error`, upstream never contacted; 2 tests |
| V3-17 | Provider integration testing | partial | local mock upstreams only; real API smoke needs owner consent |
| V3-18 | Quota classification verification | done | `docs/V3/V3-18-failover-verification.md` (failover.rs 5 tests + loopback suite) |
| V3-19 | Retry and cooldown | done | `docs/V3/V3-18-failover-verification.md` §Retry-After |
| V3-20 | Credential rotation | done | provider/credential pairing + disabled-key refusal in `proxy::resolve_secret`; 2 tests |
| V3-21 | Cross-provider fallback | done | `tests/proxy_loopback.rs` ordered fallback |
| V3-22 | Streaming safety | done | `tests/proxy_loopback.rs` no-replay test |
| V3-23 | Recovery verification | done | `docs/V3/V3-18-failover-verification.md` |
| V3-24 | Connection status accuracy | done | `lib/connectionLabel.ts` states; AppHeader + 7 tests |
| V3-25 | Direct/Routed mode safety | done | footer mode-aware action + confirm; 2 tests |
| V3-26 | Credential path correctness | done | `config_paths` command; CredentialsTab uses backend value; 2 tests |
| V3-27 | Provider/model switch polish | done | routing keeps its own credential list (`routingCredentials`) |
| V3-28 | Compact contextual help | done | Failover explanation moved into a disclosure |
| V3-29 | Window and sidebar refinement | done | min size 620×560; screenshots at both sizes |
| V3-30 | Visual/accessibility verification | partial | code-level a11y suite green; one screenshot per size; no VoiceOver run |
| V3-31 | Secret and token audit | done | `docs/V3/V3-31-security-audit.md`; token now 256-bit OS CSPRNG |
| V3-32 | Proxy security | done | `tests/proxy_security.rs` 8 tests; exact-path match fixed |
| V3-33 | Configuration safety | done | `docs/V3/V3-31-security-audit.md` §V3-33 |
| V3-34 | CI investigation | done | `docs/V3/V3-34-ci-investigation.md` — billing lock, jobs never start |
| V3-35 | Complete regression | done | `docs/V3/V3-35-regression.md` |
| V3-36 | Real macOS workflow test | not verified | helper survival verified on the release binary; live Claude Code session not performed |
| V3-37 | Packaging and cleanup | done | release build exit 0; DMG 5.93 MiB |
| V3-38 | Final documentation and audit | done | `docs/V3/FINAL_AUDIT.md`, `docs/V3/LIMITATIONS.md` |
| V3-39 | Final delivery | done | pushed to `main` |
