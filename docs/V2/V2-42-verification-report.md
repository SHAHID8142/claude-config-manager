# V2-42 … V2-51 — verification report

Code-level verification, per the owner's instruction (no screenshot loops). Every claim below
names the check that produced it; anything not checked is listed as **unverified**.

## V2-42 Direct-mode workflow

| Step | Evidence |
| --- | --- |
| Provider → key → model → Save & Apply writes exactly the managed keys | `save_apply::tests::full_pipeline_happy_path`, `apply_never_writes_a_second_model_key_or_a_failover_chain` |
| Changing only the model keeps the stored key and does not require retyping | `has_credential` command (`lib.rs::tests::has_credential_rejects_an_unsafe_provider_id_before_touching_disk`) plus the UI test `changing only the model keeps Save enabled when a key is already stored` |
| A failed apply leaves the previous working configuration in place | `save_apply::tests::failed_apply_keeps_the_previous_model_instead_of_failing_over` |
| Restore returns the newest backup and writes a pre-restore copy | `config_restore::tests::{restores_newest_backup_with_pre_restore_safety_copy, corrupted_backup_never_restored, restore_after_settings_deleted_recreates_file}` |

## V2-43 Routed-mode workflow

| Step | Evidence |
| --- | --- |
| Targets are ordered and reorderable, ≤ 10, ids unique | `routing_config` unit tests; `RoutingPanel` "moves one up / cannot move the first up" tests |
| Applying routed mode writes the loopback URL plus a token helper and no key | `save_apply::tests::routed_mode_points_claude_code_at_the_loopback_proxy` |
| End-to-end: stored key → routing target → proxy → upstream answer | `tests/proxy_loopback.rs::routed_mode_end_to_end_through_the_generated_helper` |
| Fallback happens on 429/5xx/timeout/auth, not on 400/404/422 | `falls_back_on_rate_limit_and_reports_it`, `does_not_fail_over_on_a_request_error`, `reports_all_targets_failed_without_leaking_key_material` |
| Streaming bytes pass through unchanged | `streams_the_upstream_body_through` (exact SSE body equality) |
| Routing off by default | `RoutingConfig::default` (`enabled: false`) and its `routing_config` test; `apply_routed_mode` refuses while off |

## V2-44 Credential / migration / localhost / log audit

| Check | Result |
| --- | --- |
| No key material in the routing file | `routing.json` stores credential **ids**; schema is `deny_unknown_fields` |
| No key material in proxy status or errors | `ProxyStatus` carries ids, counters and a class-based message; `reports_all_targets_failed_without_leaking_key_material` asserts the fixture key is absent from the error body |
| No `println!`/`eprintln!`/`console.log` in the new modules | `grep` over `proxy.rs`, `routing_config.rs`, `failover.rs`, `save_apply.rs`: no matches |
| Loopback http allowed only for 127.0.0.1 / localhost | `config_merge::tests::loopback_proxy_url_is_allowed_but_remote_http_is_not` |
| Legacy `<provider>.secret` files are never moved or rewritten | `credential_vault` + `credential_migration` tests; the vault aliases `<provider>-legacy` and reads the old path read-only |
| Secret files are 0600, directory 0700, writes atomic | `credential_vault` tests; routed-mode test asserts the proxy token file mode |
| Proxy token is not a provider credential and is never sent upstream | `proxy::try_target` sends only the resolved provider secret; the E2E test asserts the key is absent from the settings file and that the upstream body holds no key |

## V2-45 Dependency + supply-chain audit

`bash scripts/dep-audit.sh`:

```
npm audit: found 0 vulnerabilities
macOS dependency graph: checked 717 resolved packages, 0 affecting the shipped graph
excluded (Linux-only / build-time): glib 0.18.5 (GHSA-wrw7-89jp-8q8g, RUSTSEC-2024-0429),
proc-macro-error 1.0.4 (RUSTSEC-2024-0370)
PASS: no advisory affects a dependency of the macOS build
```

No new third-party binary, archive or licence obligation was added by this milestone: the proxy
is first-party Rust using the already-vendored `ureq`.

## V2-46 Keyboard / a11y / sizing / visual regression

| Check | Result |
| --- | --- |
| Every new control is a labelled native control | provider/credential `<select>`, model `<input>` with text labels; toggle is a real checkbox with `<label for>` |
| Icon-only buttons have accessible names | `Move <id> up`, `Move <id> down`, `Remove <id>` (asserted in the panel tests) |
| Disabled states are real `disabled`, not colour-only | first/last reorder buttons, "Route Claude Code" with an empty chain |
| Focus is visible | shared `focusRing` token (`focus-visible:ring-accent`) on inputs and selects |
| Contrast | `src/styles/theme.contrast.test.ts` (WCAG both themes) |
| Window sizing | 760×780 minimum; the new section scrolls inside the existing column |
| Visual regression | **not verified** — no screenshots taken, per owner instruction |

## V2-47 Claude Code config safety + reversibility

| Check | Result |
| --- | --- |
| Malformed `settings.json` is never overwritten | `config_reader` refuses, apply returns before backup/write; existing tests |
| Symlinked settings are refused | `LocatedStatus::Symlink` arm |
| Every write is preceded by a backup | `write_desired` → `create_backup` when the file exists |
| Stale-config conflict is detected | `write_settings_atomic(..., mtime)` |
| Reverting to direct mode is possible | saving normally with a provider rewrites `ANTHROPIC_BASE_URL` to that provider or clears it (`clear_env_base_url`); the proxy can be stopped and the helper replaced |

## V2-48 Close/reopen, helper lifecycle, uninstall

* Closing the app drops `AppState`, which drops `ProxyHandle` (`Drop` → `shutdown`) — the port is
  released and the token becomes invalid.
* The helper script is regenerated on every apply/routed start, so a stale token cannot linger
  after a restart.
* Uninstall: the app bundle is self-contained; `~/.claude-config-manager/` (helpers, secrets,
  routing, backups) and the managed keys in `~/.claude/settings.json` remain and are documented
  as the things to remove manually. Verified by inspection, not by a destructive test run.

## V2-49 Safe cleanup of project build caches

Only project-generated artifacts are eligible: `src-tauri/target/` and `node_modules/.vite`.
No `~/Library/Caches`, no user documents, no key files. Paths are checked to be inside the repo
before removal; the last cleanup used `cargo clean` inside the repo only.

## V2-50 Full gate run

| Gate | Result |
| --- | --- |
| `cargo fmt --check` | clean |
| `cargo clippy --all-targets` | 0 warnings |
| `cargo test` | **291 passed / 0 failed** (3 live tests ignored by design) |
| `npx tsc --noEmit` | clean |
| `npm run lint` | clean |
| `npx vitest run` | **177 passed / 22 files** |
| `npm run tauri build` | exit 0, `.app` + `.dmg` produced |
| `scripts/dep-audit.sh` | PASS |

## V2-51 macOS smoke test

* Built bundle installed to `/Applications`; launching it starts
  `/Applications/Claude Config Manager.app/Contents/MacOS/claude-config-manager` and it stays
  resident (checked with `pgrep`).
* Gatekeeper: the bundle is **ad-hoc signed**, not notarized — first launch on a clean Mac
  requires the documented right-click → Open. Unverified on a clean machine and on Windows/Linux
  (no access).
* Live failover against a paid provider was **not** run (would need a real charge); the loopback
  integration tests cover the same code path with a scripted upstream.
