# T11 — Typed UI↔Rust command contracts with mocks

**Date:** 2026-11-08 · **Result:** ✅ PASS

## Implementation
- **Rust (`src-tauri/src/lib.rs`):** `CommandError` serde-tagged enum (`invalid_input`/`io`/`not_available`, `{kind, message}` wire shape) — all future commands return `Result<T, CommandError>` so the UI never receives raw panic strings. `greet` upgraded to the contract with a negative guard (names >256 chars → typed error). 4 contract unit tests incl. serialization shape and negative case.
- **TS (`src/lib/ipc.ts`):** `IpcLike` transport seam (real/mock swap), `defaultIpc` over `@tauri-apps/api/core`, `CommandError` mirror + `isCommandError` guard, typed `greet()` with client-side mirror of the Rust guard (rejects before transport).
- **Mocks (`src/lib/ipc-mock.ts`):** `createMockIpc` with fixture handlers, simulated typed failures, unknown-command `not_available` rejection, and invocation recording for interaction tests.
- **Contract tests (`src/lib/ipc.test.ts`):** success path + call recording; client-side negative (oversized name, transport untouched); Rust-failure mirroring; `isCommandError` positive/negative cases; mock unknown-command rejection.

## Security review
- Errors are structured strings by contract; secret-shape content in messages is prohibited by contract comment (enforcement lands with the T23 redaction guard).
- Transport seam prevents UI code from invoking raw untyped commands (invocation outside `ipc.ts` is a documented violation).

## Tests (evidence)
- `check:frontend`: lint ✓ format ✓ typecheck ✓ vitest **8/8** ✓
- `check:rust`: fmt ✓ clippy -D warnings ✓ cargo test **4/4** ✓

## Acceptance criteria
- [x] Typed contracts with mocks
- [x] Positive + negative contract tests both sides
- [x] Pattern established for all future commands (same-task contract updates)
