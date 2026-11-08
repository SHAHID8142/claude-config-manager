# Test Strategy (T07)

**Date:** 2026-11-08 · **Binding for all implementation tasks** · Derived from PRD quality gates, threat model (T05), module boundaries (T06).

## 1. Golden rule

**The live `~/.claude/settings.json` is never touched by any test.** All tests operate on disposable fixtures in temp directories (`$TMPDIR/ccm-test-<random>/`). `ConfigWriter` and `ClaudeCodeAdapter` take explicit paths (T06) — no test constructs the real home path.

## 2. Fixture design

- **`fixture_home()`** — helper creating a fake home: `settings.json` scenarios: empty, valid-with-unrelated-keys, malformed JSON, strict-JSON violations (comments/trailing commas), pre-existing env block with redacted-looking values, read-only file (chmod), symlink target, missing file.
- **`fake_provider` fixtures** — canned provider catalog entries with fake keys (`sk-test-DO-NOT-USE-…`), never real.
- **`mock_discovery_server`** — local HTTP fixture (loopback, ephemeral port) returning documented model-catalog shapes; plus adversarial variants (huge payload, malformed JSON, wrong content-type, injection strings in model IDs).
- **`bridge_fixtures`** — pinned-version behavior fixtures: SSE streams (Anthropic and translated OpenAI shapes), tool-call request/response pairs, checksum-mismatch artifacts, tampered tarball, slow/hanging server for timeouts.
- **Secret fixtures** — fake keychains via `MockSecretStore`; the real macOS Keychain is exercised only in T22's manual-verified round-trip (test item created and deleted, name prefixed `ccm-selftest-`).

## 3. Test layers and gates

| Layer | Scope | Gate per task |
|---|---|---|
| Unit (Rust) | merge, validation, backup, atomic write, redaction, catalog parsing, checksums | `cargo test` green; every new module ships ≥1 positive + ≥1 negative test |
| Integration (Rust) | full Save & Apply pipeline on fixture home; concurrent-writer race; interrupted-write recovery | included in `cargo test` |
| Unit (TS/React) | IPC contract mocks, UI state logic, validation UI | `vitest` green |
| Component/E2E (frontend) | provider selector, masked key UX, search/no-autoselect | vitest + manual scripted checks; a11y checks in T62 |
| Security regression (T59–T61) | threat-model-derived adversarial suites | dedicated test files, all green |
| Build | `cargo check`, `vite build`, `cargo clippy -D warnings`, `tsc --noEmit` | green per task |

## 4. Per-task acceptance mapping (abridged)

Config safety tasks (T26–T36): every task must include the negative cases named in the backlog item + threat-model mitigations T3/T4/T5. Credential tasks (T20–T23): redaction tests mandatory. Bridge tasks (T47–T58): every task touching execution needs loopback + integrity tests. UX tasks (T12–T15, T62): keyboard/a11y assertions or manual checklist evidence.

## 5. CI (T10)

GitHub Actions on push/PR: install pinned toolchain → `npm ci` → lint/typecheck/vitest → `cargo test`/clippy → `npm audit` + `cargo audit` → gitleaks secret scan. No real secrets in CI (PRD); provider/bridge smoke tests run against local fixtures only.

---

# Security Checklist (T07) — run before marking any task complete

**Secrets (T1/T2/T15)**
- [ ] No real credentials anywhere: source, tests, fixtures, logs, commits (grep for `sk-ant`, `sk-or`, `nvapi-`, provider-specific prefixes)
- [ ] Error paths and log lines pass redaction guard tests
- [ ] Backups 0600 and redaction policy applied
- [ ] Reveal control auto-hides; no full-key copy paths

**Config safety (T3/T4/T5)**
- [ ] Invalid source never overwritten; malformed JSON → error, not repair
- [ ] Atomic write + read-back + rollback verified on fixture
- [ ] Concurrent-modification detection tested
- [ ] Symlink refusal + traversal + permission-denied negatives tested
- [ ] Only supported keys written; unrelated keys byte-preserved

**Injection & paths (T6)**
- [ ] No `format!`-into-shell, no `Command::new` with user strings beyond argv
- [ ] URL scheme/host validation; model-ID character whitelist

**Supply chain (T7/T8)**
- [ ] New deps justified, license checked, lockfile committed
- [ ] Bridge: pinned version + checksum verify before any execution
- [ ] Download over HTTPS from pinned upstream only

**Network/localhost (T9/T10)**
- [ ] Bridge config binds 127.0.0.1 only; 0.0.0.0 refused
- [ ] Health checks can't hang (timeouts); port conflicts surfaced
- [ ] Streaming/tool-call fixtures pass through intact

**Auth/behavior (T11/T12)**
- [ ] Never auto-select model; selection only by explicit action
- [ ] Test-connection echoes endpoint+model; no cached success
- [ ] Capability labels never fabricated

**OS (T14/T18)**
- [ ] Keychain errors handled gracefully
- [ ] Helper start/stop/status tested; GUI-closure lifecycle documented

## Regression discipline

Each hardening task (T59–T63) reruns the full prior suites. Any regression = fix before task completion (rule 6). Failures after 3 focused fix cycles → STOP with precise blocker (rule 14).
