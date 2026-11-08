# T26 — Read-only global Claude Code config locator

**Date:** 2026-11-08 · **Result:** ✅ PASS · **Artifact:** `src-tauri/src/config_locator.rs`

## Implementation
- `ConfigLocator::locate(home, config_dir_override)` → `<home>/.claude/settings.json` (user scope per T02 matrix); absolute `CLAUDE_CONFIG_DIR` honored, relative override resolved against home. Override is an explicit parameter (production reads the env once; tests race-free).
- `inspect(path)` → `Missing | RegularFile | Symlink | OtherFile` via `symlink_metadata` (no content read — strictly read-only).
- **Symlink refusal** (threat T5): reported distinctly; downstream readers/writers treat `Symlink` as an error, never a write target.

## Security review
- Path computation only — no content access, no writes. Symlink detection precedes any future read/write (T27/T30 build on this).
- Directory/fifo at the settings path surfaced as `OtherFile` (unusable state, not silently treated as a file).

## Tests (evidence)
- 7 tests: default path, absolute override, relative override vs home, regular-file detection, **symlink refusal**, directory-at-path, missing-parent (first-run state).
- `cargo test` **60/60**; clippy -D warnings clean; fmt clean.

## Acceptance criteria
- [x] Read-only locator honoring platform conventions + CLAUDE_CONFIG_DIR
- [x] Missing-file and malformed-shape states handled (report, don't guess)
- [x] Symlink negative case covered
