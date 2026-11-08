# Verification sweep of completed tasks T01–T41

**Date:** 2026-11-08 · **Requested by:** user ("check all tasks completed which are marked as done")
**Method:** evidence read from **committed git objects** (`git cat-file`, `git show`, `git log`), not from
claims in `TASK_PROGRESS.md`. Plus a full local gate run and targeted re-execution of the two tasks whose
evidence cannot be proven offline (T22 real Keychain, T38 live catalog).

## 1. Structural evidence — 41/41 pass

For every task marked ✅ the sweep asserted three things:

| Assertion | Result |
|---|---|
| `docs/audits/Tnn-*.md` exists in `HEAD` | 41 / 41 present |
| ≥ 1 commit referencing `Tnn` exists in history | 41 / 41 found |
| Primary artifact named by the task exists in `HEAD` | 44 artifact checks, **0 missing** |

`git`-derived commits (now recorded in `TASK_PROGRESS.md`, replacing "(Tnn commit)" placeholders):
`T01 586ff91 · T02 c17783f · T03 4716814 · T04 af48cdf · T05 76bd980 · T06 76bd980 · T07 0034acf ·
T08 1ed5f74 · T09 4ac0d37 · T10 35eb75a+9b46d8f · T11 5aab0bd · T12 6781523 · T13 db89b24 · T14 6493b97 ·
T15 7976f2f · T16 747c030 · T17 a16b172 · T18 ad164a2 · T19 8a41697 · T20 9b57c85 · T21 9ba4157 ·
T22 d523d3b · T23 16cc4e6 · T24 6dc26c3, c989338, af66992+d7e0ccf, ceb1780 · T25 3b1633e · T26 ae32dc1 ·
T27 fd94eb4 · T28 fa4209e · T29 d6f5dba · T30 32c0b80 · T31 6981a78 · T32 93a6db7 · T33 6bf8b12 ·
T34 84515d1 · T35 38fdb71 · T36 6c7220c · T37 17c8e4b · T38 d08f706 · T39 585e802 · T40 12118e7 ·
T41 e975258`

## 2. Dynamic evidence — claims re-executed, not trusted

| Task | Claim in progress file | Verification performed | Verdict |
|---|---|---|---|
| T22 | "real round-trip verified" | `cargo test keychain` on this macOS host: `keychain_round_trip_real`, `keychain_delete_missing_is_idempotent_real` **pass** (fake value, unique account, cleaned up) | ✅ **confirmed live** |
| T38 | "live-verified 464 models" | `cargo test live_openrouter -- --ignored`: **461 models** returned today | ✅ confirmed, **count drifts** (see §4) |
| T30/T31 | atomic writer + adversarial suite | `cargo test` incl. `tests/adversarial_paths.rs` (7) — all pass | ✅ |
| T33/T34/T35/T36 | apply/status/restore/environment | module tests executed in the same run — all pass | ✅ |
| T10 | "workflow valid; exec pending" | `ci.yml` parses; jobs `frontend`, `rust`, `security` | ✅ valid, **still never executed** (GitHub billing lock) |

Full gate run at this commit: `npm run lint` ✓ · `format:check` ✓ · `typecheck` ✓ · `test` **87/87** ✓ ·
`build` ✓ · `cargo fmt --check` ✓ · `cargo clippy -- -D warnings` ✓ · `cargo test` **143 passed / 1
ignored + 7 integration = 150** ✓

## 3. Discrepancies found and how they were handled

| # | Finding | Action |
|---|---|---|
| V1 | `TASK_PROGRESS.md` recorded "(Tnn commit)" placeholders for 35 of 41 completed tasks instead of actual hashes (rule 12 asks for the actual hash) | Hashes derived from git and written into the file (this commit) |
| V2 | Footer still said "Next task: **T02**" and "Blockers: none" | Corrected to T43; T10 CI-exec-pending already disclosed in its row |
| V3 | `keychain_store.rs` header says "keyring crate (v3)" while `Cargo.toml` pins `keyring = "4"` | Header corrected (this commit) |
| V4 | `src-tauri/src/App.test.tsx` — an 82-line React test committed by the T34 commit into the Rust source tree. Not part of T34's deliverable, not run by any tool | Removed in a separate commit (kept atomic) |
| V5 | T38's "464 models" is a moving number (461 today) | Recorded here; audit for T38 is not rewritten — its claim was true when written |

## 4. Honest caveats

- T10's CI has **never run**; all "green" statements above are **local** executions on macOS.
- T22 passes on *this* machine, but the macOS `#[cfg]` tests are not executed in the Linux CI job, so the
  primary platform's credential path has no automated regression coverage in CI.
- `cargo clippy --all-targets -- -D warnings` reports 8 pre-existing lints in **test code**; the project
  and CI gate (`cargo clippy -- -D warnings`) is green. Deferred to the T59–T63 hardening sweep.
- The sweep proves artifacts and behaviour exist; it does not re-litigate design decisions of each task.
  Acceptance-criteria review across all tasks remains T67's job.
