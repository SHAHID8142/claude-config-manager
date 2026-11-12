# V3 — Final audit: Stability, Compatibility & Final Polish

Baseline `9d17575` → this phase. Repository: `github.com/SHAHID8142/claude-config-manager`,
branch `main`.

## Verdict

**Not releasable as "V3 READY" under the phase's own exit criteria.** Two of them
cannot be met from this machine:

1. **Real macOS workflow test (V3-36)** — a live Claude Code conversation through
   the proxy in a VS Code terminal, including a simulated upstream failure, needs
   a real provider key and an interactive Claude Code session. Everything up to
   that point (helper start, survival after the GUI exits, `/healthz`, restart,
   crash recovery, routed apply) was verified.
2. **GitHub Actions** — every job fails before starting because the GitHub
   account is billing-locked. CI provides no signal; it must not be described as
   passing.

Everything else in the phase is implemented, tested and pushed. The build is
usable today for Direct Mode and for routed mode on Anthropic-compatible
providers, with the limitations below disclosed.

## Tasks

| Task | Status | Where |
| --- | --- | --- |
| V3-01 baseline | done | `V3-01-baseline.md` |
| V3-02 defect verification | done | `V3-02-defects.md` — 6 real, 1 reclassified (found deeper: cross-provider key use) |
| V3-03 regression baseline | done | `tests/v3_regressions.rs`, `src/v3-regressions.test.tsx` |
| V3-04 helper lifecycle design | done | `V3-04-helper-lifecycle.md` |
| V3-05 proxy separated from GUI | done | `src-tauri/src/helper.rs`, `main.rs` |
| V3-06 secure helper startup | done | 256-bit CSPRNG token, loopback bind, `0600` state/token, no shell interpolation |
| V3-07 lifecycle management | done | `helper_status`, `proxy_start/stop/restart`, stale detection |
| V3-08 routed configuration persistence | done | persistent port + token; `apply_routed_mode` starts the helper |
| V3-09 restart/reboot + opt-in login start | done | `helper_login_start`, LaunchAgent plist with `Umask 077` |
| V3-10 lifecycle regression tests | done | `tests/helper_lifecycle.rs` (real binary, 5 tests) |
| V3-11 compatibility matrix | done | `V3-11-compatibility-matrix.md` |
| V3-12 bridge evaluation | **blocked** | `V3-12-bridge-evaluation.md` — licence unverifiable offline; needs owner choice |
| V3-13 adapter interface | done | `proxy::Adapter`, `adapter_for()` |
| V3-14/15 translation | **blocked** | follows V3-12 |
| V3-16 capability protection | done | `501 compatibility_error`, upstream never contacted |
| V3-17 provider integration tests | partial | local mock upstreams only |
| V3-18 failure classification | done | `V3-18-failover-verification.md` |
| V3-19 retry/cooldown | done | bounded `Retry-After`, per-class cooldowns |
| V3-20 credential selection | done (+ security fix) | provider pairing + disabled-key refusal |
| V3-21 cross-provider fallback | done | ordered chain, owner's order never rewritten |
| V3-22 streaming safety | done | no replay after first byte |
| V3-23 recovery | done | cooling-skip, recovery, repeated 429 |
| V3-24 connection status accuracy | done | 7 states; `verified` only after a real test |
| V3-25 direct/routed safety | done | mode-aware footer + second confirmation |
| V3-26 credential path correctness | done | `config_paths` command |
| V3-27 provider/model switch polish | done | routing keeps its own list |
| V3-28 compact contextual help | done | disclosure instead of paragraphs |
| V3-29 window/sidebar refinement | done | 620×560 and 760×780 screenshots reviewed |
| V3-30 visual/a11y verification | partial | structural suite green; no VoiceOver/contrast run |
| V3-31 secret/token audit | done | `V3-31-security-audit.md` |
| V3-32 proxy security | done | `tests/proxy_security.rs` (8 tests) |
| V3-33 configuration safety | done | backups, atomic verify, symlink refusal, rollback |
| V3-34 CI investigation | done | billing lock, not a code failure |
| V3-35 complete regression | done | `V3-35-regression.md` |
| V3-36 real macOS workflow | **not verified** | requires a live key + Claude Code session |
| V3-37 packaging | done | release build exit 0, DMG 5.93 MiB |
| V3-38 documentation | done | this file, `LIMITATIONS.md`, `USER_GUIDE.md` §11 |
| V3-39 delivery | done | commits pushed to `main` |

## Commits

| Commit | What it did |
| --- | --- |
| `f4fc118` | baseline + defect verification + executable regression baseline |
| `57acc53` | detached proxy helper (V3-04…V3-10) |
| `8115fc0` | honest connection states, routed-mode guard, real paths (V3-24…V3-28) |
| `73b0f16` | compatibility matrix, adapter boundary, explicit bridge refusal (V3-11…V3-17) |
| final | security audit hardening (CSPRNG token, exact path match), proxy security suite, window/a11y verification, regression record, limitations, this audit |

## Executed evidence

```
cargo fmt --check                     clean
cargo clippy --all-targets -D warnings clean
cargo test                            314 passed / 0 failed (3 live ignored)
tsc --noEmit                          clean
eslint .                              clean
vitest run                            196 passed / 25 files
scripts/dep-audit.sh                  PASS
npm run tauri build                   exit 0 · DMG 5.93 MiB
```

New in V3: **23 Rust tests** (5 helper lifecycle, 8 proxy security, 6 V3
regressions, 4 helper unit) and **11 TypeScript tests**.

## Security findings

1. **Fixed — cross-provider credential use.** `resolve_secret` returned any
   credential by id; a hand-edited `routing.json` could send provider B's key to
   provider A's endpoint. Now pairs credential with provider and refuses disabled
   keys.
2. **Fixed — weak token source.** The proxy bearer token came from mixed hashers;
   it now comes from the OS CSPRNG (256 bits).
3. **Fixed — path prefix match.** `/v1/messages/../x` produced an opaque 503; the
   path is now matched exactly (404).
4. **Hardened — helper log.** launchd creates the log file; the plist now sets
   `Umask 077`.
5. **Open, accepted:** the proxy token is a disk secret readable by the owner's
   account (same trust boundary as `~/.claude`); loopback HTTP, no TLS.

## Supported providers

| Provider | Mode | Status |
| --- | --- | --- |
| Anthropic | direct + routed | Anthropic native, forwarded verbatim |
| OpenRouter | direct + routed | Anthropic-compatible gateway |
| DeepSeek | direct + routed | Anthropic-compatible gateway |
| Z.ai (GLM) | direct + routed | Anthropic-compatible gateway |
| NVIDIA NIM | — | OpenAI-only, refused with `compatibility_error` |
| Custom endpoint | direct + routed | owner asserts Anthropic compatibility |

## Release blockers

1. Real Claude Code session through the proxy (V3-36) — needs a provider key.
2. GitHub Actions billing lock — no CI signal exists.
3. Bridge decision (V3-12) if OpenAI-only providers must work in-app.
4. Apple Developer ID for a warning-free install on another Mac.
5. Project `LICENSE` still undecided.

## What changed for the owner, in one paragraph

Routed mode now survives closing the app: a detached helper keeps the loopback
proxy up, keeps the same port and token across restarts, reports a killed helper
as stale instead of pretending, and can optionally start at login through a
LaunchAgent you install with one checkbox. The header no longer claims
"connected" from a file on disk — it says configured, verified, unavailable or
proxy stopped, and only a real connection test can produce "verified". The global
Save & Apply button can no longer silently undo a working routed setup. The
Credentials screen names the directory the app actually writes. OpenAI-only
providers are refused with a clear compatibility error rather than sent a body
they cannot parse.
