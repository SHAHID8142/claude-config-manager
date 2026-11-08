# T29 — Secure backup with correct permissions + secret redaction policy

**Date:** 2026-11-08 · **Result:** ✅ PASS · **Artifact:** `src-tauri/src/config_backup.rs`

## Implementation
- `create_backup`: refuses missing/symlinked sources (threat T5); byte-exact copy verified by size before success; **0600 permissions enforced after write**; `create_new` (no clobbering); fsync best-effort; nanosecond-timestamped unique names.
- **Redaction policy (threat T15) — explicit tradeoff, disclosed:** backups intentionally contain the full file including plaintext env secrets (restore must be complete, PRD F07); the policy compensates: 0600 owner-only perms, app-owned local directory, metadata-only listings (name/size/time), content never enters logs/errors/UI (asserted by test), symlinked backup entries skipped in listings.
- `list_backups`: newest-first, only `settings-backup-*.json` entries, symlink/non-file entries skipped, missing dir → empty list.
- `default_backup_dir`: `~/Library/Application Support/ClaudeConfigManager/backups` (production; tests inject fixtures).

## Security review (checklist §Secrets/Config)
- [x] Backup-before-write established (wired into apply pipeline T30/T33)
- [x] 0600 enforced + tested; unique names tested
- [x] Metadata-only listings — no secret in any surface (tested)
- [x] Symlink refusal on source; symlink skip on listing

## Tests (evidence)
- 7 tests: byte-exact + 0600 round-trip, unique prefixed names, missing source, symlinked source, newest-first metadata-only list (secret-absence asserted), symlinked-entry skip, prefix/suffix filtering.
- `cargo test` **88/88**; clippy -D warnings clean; fmt clean.

## Acceptance criteria
- [x] Backup with correct file permissions
- [x] Redaction policy defined and tested (local-only, never-logged)
