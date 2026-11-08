# T35 — Restore-last-backup operation + tests

**Date:** 2026-11-08 · **Result:** ✅ PASS (1 focused fix cycle) · **Artifacts:** `src-tauri/src/config_restore.rs`, `restore_last_backup` command, TS contract + UI button

## Implementation
- `restore_last_backup(settings_path, backups_dir)` — fail-closed pipeline: newest backup via the strict T29 listing (symlinks/traversal never surface) → **content gate before any write** (strict JSON + object root; corrupted backups refused, current state untouched — asserted) → **pre-restore safety backup** of the current state (restore is itself a write and must be reversible; skipped with honest empty marker when the file is missing) → atomic write via the T30 writer (0600, fsync, mtime conflict check, read-back verification).
- Note: restore normalizes formatting (pretty JSON, value-equal to the backup) — tests verify semantic equality, not byte equality; documented in code.
- **Command + UI:** `restore_last_backup` command; TS contract; "Restore last backup" `LoadingButton` in Actions with success banner (restored-from + pre-restore copy names) and error banner.

## Security review
- Corrupted/malformed backups are never applied (content gate precedes any mutation; tests assert current state byte-intact after refusal).
- Pre-restore copy is 0600 (asserted); no secret content in any banner — only file names.

## Tests (evidence)
- Rust **122/122** + 7 integration (5 new: newest-first restore with safety copy, no-backups refusal, corrupted-backup refusal, non-object refusal, deleted-file recreation).
- Frontend **67/67** (2 new: success flow with both file names, error flow).
- clippy -D warnings clean; fmt clean.

## Fix cycle (1 of 3)
- Test assertions compared raw strings; the writer's pretty-print normalization is intended behavior — switched to parsed-value comparison + removed a clippy-flagged no-op `drop`.

## Acceptance criteria
- [x] Restore-last-backup with reversible pre-restore protection
- [x] Refusal paths (no backups, corrupted, non-object)
- [x] UI wired with honest feedback
