# Threat Model (T05)

**Date:** 2026-11-08 · **Scope:** Claude Config Manager v1 (macOS-first) · **Method:** STRIDE-informed enumeration ranked by risk (likelihood × impact), each mapped to binding mitigations and backlog tasks. Owned by tasks T07 (checklist), T59–T63 (regression/audit).

## Assets

- **A1** User API credentials (per-provider keys/tokens; in Keychain, in settings.json `env`, in memory)
- **A2** Global `~/.claude/settings.json` (user's working config — corruption = broken CLI)
- **A3** App-owned data: backups, bridge binary, bridge config, app config
- **A4** Local bridge network surface (127.0.0.1 ports)
- **A5** User trust artifacts: logs, diagnostics, UI display

## Trust boundaries

TB1 UI ⇄ Rust core (IPC) · TB2 App ⇄ filesystem (`~/.claude`, app dir) · TB3 App ⇄ provider APIs (HTTPS) · TB4 App ⇄ GitHub/npm release distribution (bridge download) · TB5 Claude Code CLI ⇄ bridge ⇄ provider · TB6 App ⇄ macOS Keychain.

## Threat register

| ID | Threat | Vector | Sev | Mitigations (tasks) |
|---|---|---|---|---|
| **T1** | **Credential exfiltration via settings.json plaintext** | `env.ANTHROPIC_API_KEY`/`AUTH_TOKEN` written plaintext (0600); attacker with user-level read can steal keys | HIGH | Keychain default storage (T21–T22); plaintext only via `apiKeyHelper` (T32) which reads keychain at CLI runtime, or with explicit consent + disclosed tradeoff; file permission enforcement (T29); redaction everywhere (T23) |
| **T2** | **Secrets in logs/diagnostics/backups/errors** | keys leak via error messages, bridge logs, backup copies | HIGH | Central redaction guard (T23); no raw credentials in logs (F08); backup redaction policy (T29); diagnostics sanitizer (T57) |
| **T3** | **Config corruption** (malformed JSON, partial write, crash mid-write, invalid source) | power loss, concurrent edit, bug in merge | HIGH | Backup-before-write (T29); atomic temp+fsync+rename (T30); read-back verification + rollback (T30); never overwrite invalid source (F02, T27); strict-JSON emit (T02 matrix) |
| **T4** | **Race conditions on config file** | CLI or another tool edits settings.json while app writes | HIGH | mtime/content conflict detection (T30); refusal on concurrent modification; single-writer discipline (T30) |
| **T5** | **Symlink/path-traversal attacks** | `~/.claude/settings.json` replaced by symlink to sensitive file; crafted paths in backup/restore | HIGH | Symlink refusal (T31); canonical path validation; traversal-negative tests (T31); app-owned backup dir with fixed layout |
| **T6** | **Shell/argument injection** | user-controlled strings (endpoint URL, model ID, helper path) interpolated into shells | HIGH | No shell interpolation with user input (architecture §threats); commands built as argv arrays; `apiKeyHelper` script is app-generated with fixed content, not user-editable strings (T32); URL/ID validation (T19, T43) |
| **T7** | **Poisoned bridge download** (supply chain) | compromised release artifact / MITM / typosquatted package | HIGH | Exact version pin + SHA-256 manifest recorded at pin time (ADR-001, T49); verify-before-execute (T49); HTTPS-only; app-owned install dir (T50); no registry auto-install |
| **T8** | **Compromised dependency** (cargo/npm) | malicious transitive dep | MED-HIGH | Lockfiles committed (T08/T09); `cargo audit` + `npm audit` in CI (T10); license/provenance audit (T63); minimal dependency count |
| **T9** | **Localhost abuse** (untrusted local process hits bridge) | any local process can reach 127.0.0.1 ports | HIGH | Loopback-only bind enforced in config (T51); bind-0.0.0.0 refused by our writer (T51 negative tests); port conflict detection; CCR mgmt UI never exposed by us (T52) |
| **T10** | **Bridge silent capability stripping** | proxy drops tool calls/vision/thinking/metadata → degraded sessions blamed on provider | MED | Fixture tests for SSE + tool-call pass-through (T55); vision/reasoning paths (T56); per-provider known-losses doc (T47); capability labels (T25) |
| **T11** | **Untrusted endpoints / malicious model metadata** | user-pasted endpoint exfiltrates credentials; provider catalogs lie about models | HIGH | HTTPS scheme validation (T19); warning on non-Anthropic hosts (T19); no invented capability values (F12, T42); discovery responses schema-validated (T37/T59) |
| **T12** | **Misleading connection tests** | "success" from stale cache, wrong endpoint, or auth-less ping | MED | Test-connection hits real provider endpoint with real credential header; result includes endpoint+model echoed; never caches success (T33/T36) |
| **T13** | **Provider billing/quota errors surfaced as app failure** | 402/429 provider states misread as config errors | LOW-MED | Error classification (T57) distinguishing provider state vs config error; actionable messages |
| **T14** | **OS integration failures: Keychain denial/lock; helper survives/unexpected start** | Keychain write refused; helper outlives expectations | MED | Keychain error handling (T22); documented helper lifecycle + tested start/stop/uninstall (T52, T53, T66); no launchd/LaunchAgent auto-start without consent (T53) |
| **T15** | **Backup secrets at rest** | backup files copy plaintext env keys | HIGH | Backups stored 0600 in app-owned dir (T29); redaction policy: backups never written to logs/cloud; restore path re-validates permissions (T35) |
| **T16** | **IPC command abuse in-app** | malformed arguments to Tauri commands from compromised webview | MED | Typed command contracts (T11); server-side revalidation in Rust (never trust UI input); capability policy enforced in Rust layer |
| **T17** | **Shoulder-surf / clipboard leak of API key** | reveal control leaves key visible; clipboard retention | LOW | Auto-rehide reveal (T20); no copy-to-clipboard of full keys; masked display default |
| **T18** | **GUI-closed helper breaks active CLI session** | user closes app, bridge dies, session hangs | MED | Standalone-run lifecycle design + tests (T53, T66); explicit user-visible status; opt-out documented |

## Residual risk statement

Accepted residual risks (documented, not silently accepted): plaintext credential presence in `~/.claude/settings.json` when user consents to env-var mode (T32 disclosure); bridge component's large attack surface minimized by pinned version + loopback + no-UI policy but not eliminated; unsigned test build in T64 if no signing credentials (documented limitation, not production claim).

## Review cadence

Threat register re-reviewed at T59–T63 (regression + audits) and before FINAL_AUDIT (T69). New threats found mid-backlog are appended here with the introducing task's audit entry referencing them.
