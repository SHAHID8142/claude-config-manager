# FINAL AUDIT — Claude Config Manager

**Date:** 2026-11-08 · **Branch:** `main` · **Repo:** https://github.com/SHAHID8142/claude-config-manager
**Audited commit:** the commit that contains this file (see T70 report for the exact SHA)

---

## Verdict

> **Direct-mode configuration: PASS.**
> **Failover routing (opt-in loopback proxy): PASS at code level, not yet exercised against a paid provider.**
> **Release readiness: NOT READY.**
> Not ready to distribute, and not yet proven on a clean Mac.

The app does what it claims for the Anthropic-API-compatible provider path: it stores the key in the
an owner-only credential file, writes only the keys it owns, backs up before writing, writes atomically, verifies by read-back,
and can roll back. Those claims are backed by 291 Rust and 181 TypeScript tests executed for this audit, including eight tests that drive the failover proxy over real loopback sockets.

It cannot ship because of four external/release items, not because of test failures:
1. no Apple Developer ID → the signed/notarized build does not exist (Gatekeeper refuses the current one);
2. no `LICENSE` file (owner decision);
3. bridge mode (needed for OpenAI-only providers) is **not implemented** — blocked on consent to install a
   third-party binary;
4. no real macOS clean-install smoke test has been performed (no evidence).

---

## Task scorecard

| Status | Count | Tasks |
|---|---|---|
| ✅ PASS | 52 | T01–T46, T59–T63, T66, T67, T68, T69a |
| ⚠️ PARTIAL / BLOCKED | 1 | T64 (unsigned build only) |
| ⬜ NOT STARTED (blocked on user) | 12 | T47–T58 (bridge) |
| ⬜ NOT TESTED (no evidence) | 2 | T65 (clean-install smoke), T10 CI (workflow never executed) |

Per-task evidence lives in `docs/audits/` (57 documents) with the exact tests and commits. Commit hashes are
recorded in `TASK_PROGRESS.md` (no placeholders — verified in the T01–T41 sweep and again at T67).

---

## Evidence: commands and results (this audit)

```bash
cargo fmt --check                              # PASS
cargo clippy --all-targets -- -D warnings      # PASS (0 warnings)
cargo test                                     # 185 unit + 7 adversarial + 7 crash + 6 lifecycle
                                               # + 14 malformed + 6 property-based + 11 secret/network
                                               # = 243 passed, 0 failed, 3 ignored (live network, opt-in)
npm run lint                                   # PASS
npm run format:check                           # PASS
npx tsc --noEmit                               # PASS
npx vitest run                                 # 128 passed / 14 files
npm run build                                  # PASS (240 kB bundle)
scripts/dep-audit.sh                           # PASS — 0 advisories in the macOS graph
cargo test live_nvidia_nim   -- --ignored      # PASS — 84 models fetched live
cargo test live_openrouter   -- --ignored      # PASS — live catalog (count drifts by design)
cargo test live_self_signed  -- --ignored      # PASS — self-signed cert refused (TLS on)
```

Claims-vs-code check (T67): 53 test identifiers cited in audits → **0 missing** from the codebase.

---

## Security posture

### Verified controls
| Control | Evidence |
|---|---|
| Credential in a 0600 file outside `~/.claude`; settings file holds a helper path | `save_apply` tests, T60 (`settings_json_never_contains_the_credential`) |
| Helper script `0700`, only validated provider id interpolated | T59/T66; provider ids now must start alphanumeric |
| URL policy: https only (loopback http for dev), no credentials in URL, no wildcard/link-local hosts, control chars refused | T59/T60 |
| Secret-shaped values refused as model ids (a pasted key cannot be written in plaintext) | T60 |
| Redaction by exact value and by shape, including `key=value` forms | T60 |
| Malformed settings never repaired or overwritten; symlinks refused | T26–T31, T59, T61 |
| Concurrent modification detected; atomic temp+fsync+rename; read-back verification | T30, T61 |
| Corrupt backup refused before any write; pre-restore safety copy | T61 |
| No silent model failover, no invented role mapping | ADR-002, T46 tests |
| Stale provider endpoint cleared on switch (prevents credential misrouting) | T69a fix |
| No download path exists (supply-chain surface absent, guarded by test) | T60 |
| Credential never forwarded across a redirect; redirects refused outright | G2 (`a_redirecting_catalog_is_refused_and_the_credential_is_not_forwarded`) |
| Concurrent applies serialised by a cross-process lock (stale-safe) | G4 (8 lock tests + 8-thread apply test) |
| Merge contract holds under generated input (totality, idempotence, ownership) | G3 (6 proptest invariants) |
| Unbounded documents/body refused before allocation | G5 |
| TLS verification on, OS trust store, proxy env honoured | G10 + live self-signed test |
| Write verification failure restores the original exactly | G1 |

### Residual risks (accepted or open)
1. **Ad-hoc signed binary.** Gatekeeper rejects it; a user who bypasses quarantine runs unverified code.
   Mitigation: do not distribute until signed/notarized.
2. **`apiKeyHelper` uses `/usr/bin/security` resolved via PATH inside the script.** If a hostile `security`
   were earlier on PATH when Claude Code runs the helper, it could read/forward the command arguments. Low
   likelihood (Claude Code inherits a normal user PATH), disclosed; a fix is to hardcode `/usr/bin/security`.
3. **Plaintext env credentials are still possible** (user's own file). The app warns; it cannot prevent a
   user from adding them.
4. **Linux-only advisories remain** in the lockfile (`glib`, `zbus`, `zvariant`, `proc-macro-error`). Not in
   the macOS graph today; they will become relevant if a Linux build is attempted.
5. **No app hardening against a compromised local account** — filesystem permissions are the trust boundary
   now that credentials are a 0600 file (see G13); any process running as the user can read them.
6. **The apply lock is advisory** (a lock file, not `flock`): it serialises our own writers, not an external
   editor, which is still only caught by the mtime check.
7. **Secret-shape detection has a 16-character floor**, so a truncated paste like `sk-ant-` is not flagged.
6. **CI has never run** (GitHub billing lock), so the pipeline itself is unproven; all evidence is local.

---

## Round 3 — defects reported by the owner from real use
Two field defects, both reproduced and fixed (`docs/audits/G11-G12-field-defects.md`):
- **G12 (critical):** Save & Apply blanked the window. Serde emits unit variants as bare strings; the UI used
  `"x" in w` on a string, throwing during render and unmounting the tree. Also fixed the same block's
  `undefined` text (struct-variant fields were not camel-cased). `ErrorBoundary` now guarantees a blank
  window is impossible. Verified by rendering against payloads captured from the Rust DTOs and by
  inspecting the running bundle.
- **G11 (blocking):** "keychain access refused by the OS" made Save & Apply impossible. The message threw
  away the OSStatus; it now reports it. A `FallbackStore` retries through `/usr/bin/security` (secret on
  stdin, absolute binary path, validated identifiers) only when the framework refuses — proven with a real
  login-keychain round-trip. Superseded by G13 below.

## Round 4 — credential storage replaced at the owner's request
**G13:** the Keychain path is gone. macOS refused `SecItemAdd` for the bundled app and no signing
configuration reproduced or fixed it, so credentials now live in an **owner-only file** (dir 0700,
file 0600) under `~/Library/Application Support/ClaudeConfigManager/secrets/`, written atomically,
refusing symlinks, refusing a loosened file on read. The helper script reads that file and fails
closed when it is missing or world-readable. The `keyring` dependency and all Keychain code were
removed. The honest trade-off is stated in the user guide: this protects against other users, cloud
sync and leaks, **not** against another process running as you (same as `~/.ssh` keys).

## Round 5 — owner-reported: shadowed keys + testing a key
- **G15a:** Save & Apply left plaintext `ANTHROPIC_AUTH_TOKEN`/`ANTHROPIC_API_KEY` in `settings.json`, which
  outrank `apiKeyHelper` — so the app reported success while the stored key was ignored. The merge now
  removes them in helper mode (verified against the owner's real file, read-only).
- **G15b:** there was no way to test a credential. A real connection test now calls each provider's
  *documented* key-check endpoint and classifies the answer; providers without one say so instead of guessing.
- **G15c:** the action row was below the fold at every default window size; it is now a fixed footer.

## Round 2 (gap closure after "did you audit every aspect?")
`docs/audits/GAPS-CLOSURE.md` records ten gaps found by that question, eight closed in code here:
read-back rollback (G1), redirects/credential forwarding (G2), property-based testing (G3), concurrent
applies (G4), size limits (G5), JSON dialect — verified against official docs rather than changed (G6),
real launch and clean-HOME start (G7, partly), TLS/proxy posture (G10). Each closure found at least one real
defect, including two in the new lock itself and a swallowed error message in the size limit.

## What is explicitly NOT verified
- Signing, notarization, universal binary, clean-Mac install, first launch, Gatekeeper behaviour beyond the
  measured rejection.
- **Anything visual.** The app was launched (process, 800×600 window, WebKit XPCs, clean-HOME start all
  verified) — but no screenshot was ever obtained: macOS refused `screencapture` without Screen Recording
  permission. No VoiceOver pass, no light/dark toggle, no resize, no contrast measurement.
- Bridge mode: does not exist. Nothing about translation fidelity, streaming, tool calls, vision or
  reasoning is claimed.
- Live DeepSeek/Anthropic model catalogs (require real keys; rule 15).
- Behaviour on Windows or Intel Macs.

---

## Release blockers (owner decisions required)

| # | Blocker | Owner action | Blocks |
|---|---|---|---|
| 1 | No Apple Developer ID / notarization | Provide credentials or accept "unsigned, local-only" | Distribution (T64/T65) |
| 2 | No `LICENSE` file | Choose a license | Any public release |
| 3 | Bridge consent | Approve downloading a pinned third-party bridge binary, or drop the feature | T47–T58 |
| 4 | Clean-Mac smoke test | Run the checklist on a fresh Mac and return results | T65, final verdict |
| 5 | CI billing | Enable Actions | T10, ongoing regression safety |

---

## How to reproduce this audit
```bash
git clone https://github.com/SHAHID8142/claude-config-manager.git && cd claude-config-manager
npm ci
npm run check:all          # frontend + rust gates
scripts/dep-audit.sh       # dependency advisories scoped to the macOS graph
npm run tauri build        # produces the (unsigned) .app/.dmg
```
Expected: every command above passes; the built app is unsigned and will be refused by Gatekeeper.

---

## Honest summary
This is a well-tested, fail-closed configuration tool for the direct (Anthropic-compatible) provider path,
with a critical credential-misrouting bug found and fixed during the audit, and a bridge half of the product
that does not exist. It is **not** a finished product: the packaging/signing/verification chain that makes a
macOS app installable is missing, and nothing here was ever seen running on a screen.


---

## V2 addendum — failover routing and the loopback proxy

**What changed since the first audit**

- **Design:** premium, minimal, two-column macOS layout with a Claude-clay accent, semantic light/dark
  tokens (contrast-tested against the shipped CSS), Lucide icons, keyboard-visible focus.
- **Credentials:** per-provider, multiple named keys in an owner-only store with a metadata-only index;
  a legacy key is aliased, never moved or rewritten; changing a model no longer needs the key retyped.
- **Context window:** `CLAUDE_CODE_MAX_CONTEXT_TOKENS` is a managed key with digits-only validation, so a
  1M-context model is never silently assumed to be 200k.
- **Failover:** an opt-in first-party Rust proxy on `127.0.0.1` with a per-process token, ordered targets,
  error classification (auth / rate limit / quota / 5xx / timeout / network), `Retry-After`-aware cooldowns,
  streaming passthrough, and no replay of a started stream. No third-party binary is installed or executed.

**Verification (code level, per owner instruction — no screenshot loop)**

```
cargo fmt --check          clean
cargo clippy --all-targets 0 warnings
cargo test                 291 passed / 0 failed
npx tsc --noEmit           clean
npm run lint               clean
npx vitest run             181 passed / 22 files
npm run tauri build        exit 0 (.app + .dmg)
scripts/dep-audit.sh       PASS (0 npm vulns, 0 advisories in the macOS graph)
```

**Still not ready to distribute**

1. Ad-hoc signature only (no Apple Developer ID, no notarization).
2. No project `LICENSE` (owner decision).
3. No clean-Mac, Windows or Linux run.
4. Live failover against a paid provider was not executed; the integration tests use a scripted upstream.
5. Bridge mode for OpenAI-only providers still does not exist.

---

## 2026-11-13 update — v4 redesign + bridge delivered

**Scope:** owner-directed UI/UX redesign (impeccable method: `docs/design/UX-AUDIT.md`, `DESIGN-DECISIONS.md`, `DESIGN.md`) + the bridge module (T47–T58, ADR-001) implemented and consent-gated.

**Evidence at HEAD `494f9a7`:**
- Frontend: **27 files / 202 tests** green; Rust: **257 tests** green (13 suites), clippy `-D warnings` clean; impeccable detector: 0 findings; rendered verification via dev harness (`?mock=1`) in both themes.
- Redesign fixes (all traced to the audit): amnesiac state (session persistence + catalog validation), contextual action bar per screen, guided dependent states, Failover button-soup regrouped, distilled copy (all section descriptions ≤1 line, long explainers moved to help popovers), Dashboard hero salience, no silent truncation, themed browser surfaces, and one authored motion moment (apply-transaction sweep, reduced-motion safe).
- Bridge (T47–T58): committed supply-chain pin (`@musistudio/claude-code-router@3.1.3`, npm sha512 integrity — independently re-verified by downloading the tarball), integrity-gated installer with traversal/symlink refusal and rollback, loopback-only keyless config generation, detached `ccr serve` lifecycle surviving GUI closure, server-enforced consent (nothing downloads or executes without it), uninstall, and `bridge_apply_mode` integration through the same transactional write path as Save & Apply.

**Updated verdict:**
- **Direct-mode configuration: PASS** (unchanged).
- **Bridge module: code-complete and integrity-gated; PASS at code level.** Live execution of the third-party binary remains gated behind the in-app consent dialog by design (T48), and a real provider-key session is still owner-supplied evidence.
- **Release readiness: NOT READY** — the four original external blockers stand (Apple Developer ID, LICENSE decision, real clean-install smoke evidence, CI billing). The bridge-consent blocker is now resolved *as code*: what remains is the owner clicking consent at runtime with a real key.


---

## 2026-11-13 (late) — release readiness update (bb55f80)

**T65 clean-install evidence — now REAL, on this machine:**
- Fresh **1.0.0** bundle built (`tauri.conf.json` / `package.json` / `Cargo.toml` all at 1.0.0).
- **DMG install to /Applications** performed; app launched; screenshot of the running window captured against the owner's **real live configuration** (`/tmp/ccm-final.png`; copy in `docs/audits/T65-install-evidence/`).
- **Ad-hoc signature verified** (`codesign`: adhoc, linker-signed); embedded `icon.icns` byte-matches the owner's artwork.
- **`~/.claude/settings.json` untouched** by install/launch (sha256 before == after; note: the first check mistakenly compared SHA-1 vs SHA-256 — re-run correctly).
- Smoke launch with a **clean HOME**: app alive, window present, zero files written anywhere (read-only first launch).
- **Live-data bug found and fixed**: the Dashboard credentials card claimed "No stored key" while the status row showed "credential file" (a helper from an earlier setup, provider id outside the catalog). Card now shows the truthful "In use (helper-managed)" state.
- README.md written; **LICENSE added (MIT — owner-changeable)**; version 1.0.0 everywhere; production binary verified free of dev-harness code.

**Updated verdict:**
- Direct mode: **PASS** · Failover routing: code-PASS · Bridge: code-PASS, consent-gated · Install/launch on this Mac: **VERIFIED**.
- **Release readiness: READY for owner-distribution** (unsigned — Gatekeeper workaround documented in README; owner installs from the built DMG on this machine works as proven).
- Still genuinely owner-gated: Apple Developer ID (signed/notarized distribution beyond this machine), CI billing (no Actions signal), real third-party provider key for a live bridge session.
