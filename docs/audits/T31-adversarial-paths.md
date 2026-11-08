# T31 — Symlink/path-traversal and permission-denied negative tests

**Date:** 2026-11-08 · **Result:** ✅ PASS (one focused hardening fix, cycle 1 of 3) · **Artifact:** `src-tauri/tests/adversarial_paths.rs` + locator/reader/writer/backup hardening

## Implementation
- **New hardening found by writing this suite:**
  1. `ConfigLocator::config_dir_is_symlink` — a symlinked config dir (`~/.claude` → elsewhere) would redirect reads AND writes outside the expected location. Reader and writer both refuse now. Deeper ancestors (automounted home dirs, sandbox temp dirs) are legitimately symlinked and deliberately not checked (documented scope).
  2. `config_backup::is_valid_backup_name` — listings now enforce the strict `settings-backup-<digits>.json` contract; traversal-shaped or decorated names (`1..2`, `1;2`, directories wearing our name) can never surface.
- Consolidated integration suite (`tests/adversarial_paths.rs`, 7 cross-stage tests on shared fixtures):
  - symlinked settings file refused at **every stage** (locate/read/backup/write) with the victim file untouched;
  - symlinked config dir refused for read+write, target dir never written;
  - traversal-shaped backup entries filtered (positive control included);
  - chmod-000 file read → `PermissionDenied`;
  - unwritable backup dir → clean error;
  - **read-only config dir** → clean failure, original intact, zero temp residue;
  - rollback recovers from simulated interrupted-write state.

## Security review
- Both new checks are fail-closed and tested on both the read and write paths.
- The suite itself initially exposed a test-authoring bug (`fs::write` with `..` resolves at OS level) — replaced with genuine adversarial names, and turned the discovery into the strict-name hardening above.

## Tests (evidence)
- 7 integration tests (new) + all 95 unit tests green; `cargo test` **102/102** total; clippy -D warnings clean; fmt clean.

## Acceptance criteria
- [x] Symlink negative tests (file-level AND directory-level)
- [x] Path-traversal shape tests (listing contract)
- [x] Permission-denied negative tests (read, backup dir, write target)
