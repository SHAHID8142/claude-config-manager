# T61 — Config rollback and interrupted-write recovery

**Date:** 2026-11-08 · **Result:** ✅ PASS (no new defects; crash paths already safe, now proven) ·
**Artifact:** `src-tauri/tests/crash_recovery.rs` (7 cases)

## What a crash can leave behind — and what the app does
| Crash point | State on disk | App behaviour | Test |
|---|---|---|---|
| After temp write, before rename | live file intact + orphan `.<name>.ccm-tmp-<nanos>` | reads live file; orphan ignored, **never** treated as settings; later apply succeeds | `orphan_temp_from_a_crash_is_ignored_and_harmless` |
| During temp write (ENOSPC / unwritable dir) | live file intact | write fails, **no temp left behind**, original bytes unchanged | `interrupted_write_leaves_no_temp_and_keeps_the_original` |
| Between snapshot and rename (another editor) | other editor's file | `ConcurrentModification`, other write not clobbered | `concurrent_edit_between_snapshot_and_write_is_refused` |
| After a bad write, manual rollback | — | exact original bytes restored, mode `0600`, no temp debris | `rollback_restores_exact_bytes_with_owner_only_permissions` |
| Backup corrupted during creation | corrupt newest backup | restore **refuses** before writing, live file untouched, no pre-restore backup created | `restore_refuses_a_corrupted_newest_backup_without_touching_settings` |
| Settings corrupted by an external crash | invalid JSON live file | valid backup restored, damaged state preserved as pre-restore safety copy | `restore_recovers_a_valid_backup_after_a_crash_damaged_settings` |
| No backups at all | — | clear `NoBackups` error (no silent no-op) | `no_backups_present_is_reported_clearly` |

## Verified properties
- **No partial state is ever observable**: every write goes through temp → `fsync` → `rename`, and the
  target is only ever the renamed file (the failure test proves the original survives a mid-pipeline
  permission failure).
- **No data loss on restore**: even a corrupt live file is copied to a pre-restore backup *after* the
  content gate passes (so a refused restore creates nothing).
- **Corrupt backups are refused by content, not by name**: a truncated newest backup cannot be installed
  over a healthy settings file.
- **Permissions**: restored/rolled-back config is `0600` (owner-only), matching the write path.

## Security review
- Rollback/restore paths re-use `atomic_write` (same 0600 + fsync + rename discipline), so a recovery
  action cannot introduce a looser-permission file.
- The corrupt-backup refusal happens **before** the pre-restore backup and before any write — a hostile or
  damaged backup cannot be used to make the app overwrite a healthy configuration.
- No secret material is read, written or logged by these paths (backups contain `apiKeyHelper` paths, not
  credentials — Keychain holds the key).

## Tests (evidence)
- `cargo test --test crash_recovery` → **7 passed**
- Full suite: **156 unit (+2 ignored) + 7 adversarial + 7 crash-recovery + 14 malformed + 11 secret/network
  = 195** · `cargo fmt --check` ✓ · `cargo clippy --all-targets -- -D warnings` ✓

## Acceptance criteria
- [x] Rollback covered: exact bytes, permissions, post-rollback validity
- [x] Interrupted-write recovery covered: orphan temp, failed write, concurrent edit
- [x] Restore hardened: corrupt backup refused before any write
- [x] Real filesystem failure (unwritable directory) exercised, not simulated with mocks

## Known limitations
- **Crash during `rename` itself is not directly simulated** — POSIX rename is atomic, so the only
  observable states are "old file" or "new file", both covered. A power-loss test (fsync of the *directory*
  entry) is not performed: the app fsyncs the temp file but does **not** fsync the parent directory, so on
  some filesystems a sudden power loss could lose the rename. Disclosed as a limitation; low impact because
  the previous backup remains restorable.
- Orphan temp files are never garbage-collected automatically; they are ignored and harmless but accumulate
  after repeated crashes.
- Restore always picks the newest *readable/valid* backup by filename order; it does not attempt to find an
  older valid one automatically when the newest is corrupt (the user can delete the corrupt file).
