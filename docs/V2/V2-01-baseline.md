# V2-01 — Baseline inspection (evidence)

**Date:** 2026-11-08 · **Branch:** `main` · **HEAD at start:** `e6e6bf8`

## Repository state
- Working tree: clean except one untracked file, `Claude Config Manager Icon.png` (owner's branding
  reference, 1254×1254, alpha channel). Preserved verbatim as
  `assets/Claude Config Manager Icon.png`; the root copy is left untouched.
- Existing docs read for this phase: `00-START-HERE.md`, `01-PRD.md`, `02-ARCHITECTURE-SECURITY.md`,
  `03-AGENT-RULES.md`, `04-TASK-BACKLOG.md`, `06-REVIEW-CHECKLIST.md`, `07-MASTER-AUTONOMOUS-PROMPT.md`,
  `FINAL_AUDIT.md`, `TASK_PROGRESS.md`, `docs/audits/*` (T41–T70, G1–G16).

## Baseline gates (run, not assumed)

| Gate | Command | Result |
| --- | --- | --- |
| Rust unit + integration | `cargo test` | **246 passed, 0 failed** (3 ignored: live network tests) |
| TypeScript | `npx vitest run` | **130 passed, 14 files** |
| Clippy | `cargo clippy --all-targets -- -D warnings` | 0 errors |
| Release bundle | `npm run tauri build` | exit 0 (built during G16) |

## Code inventory (relevant to V2)

| File | Lines | Role |
| --- | --- | --- |
| `src/App.tsx` | 698 | single-page form: provider, API key, model, advanced, status |
| `src/lib/ipc.ts` | 221 | typed IPC contract (the only place `invoke` is called) |
| `src-tauri/src/provider_catalog.rs` | 543 | provider descriptors, endpoints, auth style |
| `src-tauri/src/presets.rs` | 199 | built-in provider presets |
| `src-tauri/src/file_secret_store.rs` | ~230 | `~/.claude-config-manager/secrets/<provider>.secret`, 0600/0700 |
| `src-tauri/src/credential_service.rs` | 182 | credential read/write/clear + redaction |
| `src-tauri/src/secret_store.rs` | 233 | `SecretStore` trait + error taxonomy |
| `src-tauri/src/save_apply.rs` | — | fail-closed apply pipeline (lock → validate → secret → helper → merge → backup → atomic write) |
| `src-tauri/src/environment_check.rs` | — | CLI detection, settings state, shell-export probe |
| `src-tauri/src/connection_test.rs` | — | live credential verification per provider |

## Icon
`assets/Claude Config Manager Icon.png` — 1254 px square, RGBA. Current shipped icons in
`src-tauri/icons/` are the Tauri defaults and must be regenerated from this file (V2-17).
