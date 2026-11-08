# T36 — Failure handling: missing CLI, incompatible versions, invalid settings

**Date:** 2026-11-08 · **Result:** ✅ PASS (1 focused fix, cycle 1) · **Artifacts:** `src-tauri/src/environment_check.rs`, `check_environment` command, App environment banners

## Implementation
- **CLI detection:** PATH scan (argv-free), then `claude --version` argv-only (no shell, no user input — threat T6), poll-based 5 s timeout with kill.
- **Honest versioning (no invented requirements, T02 discipline):** version reported when parseable; unparseable output → "CLI found but version output was unparseable" warning; nothing is claimed incompatible without evidence.
- **Settings diagnostics:** reuses T26/T27 machinery — states `ok | first_run | malformed_json | not_a_file | symlink | unreadable` with detail strings that never echo file content (asserted).
- **Writable check:** probe-file in config dir, cleaned immediately (residue asserted); missing/unwritable dir reported.
- **Command + UI:** `check_environment` command; TS contract; App shows a warning banner for missing CLI ("settings can still be managed here…") and an error banner for malformed/symlinked settings ("a problem this app will not overwrite") — matching the never-overwrite contract.

## Security review
- No shell invocation; probe timeouts prevent hangs; PATH lookup parameterized for race-free testing (no env mutation — the initial thread-based env approach was removed as unsafe under edition-2024 `set_var`).

## Tests (evidence)
- Rust **128/128** + 7 integration (6 new: parser positive/negative, PATH-scan miss, malformed-settings reporting without content leak, unwritable dir, probe cleanup).
- Frontend **70/70** (3 new: CLI-missing banner, malformed-settings banner, healthy-silent case; call-order updated).
- clippy -D warnings clean; fmt clean.

## Fix cycle (1 of 3)
- Removed process-global `PATH` mutation from a test (unsafe in edition 2024, racy) in favor of a parameterized `find_claude_cli_in_path`.

## Acceptance criteria
- [x] Missing CLI handled with actionable, non-blocking guidance
- [x] Unparseable versions reported honestly, no invented compatibility claims
- [x] Invalid settings surfaced as never-overwrite states
