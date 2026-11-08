# Autonomous Coding Agent — Binding Rules (v0.2)

1. Read the entire PRD, architecture/security document, complete backlog, review checklist and master prompt before editing. Do not ask for a new task prompt between backlog items.
2. ONE atomic task at a time, in backlog order. Do not combine tasks or make unrelated changes. Keep architecture modular, avoid speculative abstractions.
3. Confirm workspace path and GitHub remote before edits. If absent, request once at startup. Never assume a directory or push to an unverified remote.
4. Maintain `TASK_PROGRESS.md` with task status, checks, commit hash, push status, blockers and next task. Resume from the first incomplete task after interruption; do not redo completed work.
5. For EACH task: inspect → brief plan → implement → unit/negative tests → lint/typecheck/build as applicable → security/adversarial review → UI/accessibility review if applicable → fix and retest → audit report → commit → push → continue automatically.
6. Do not mark a task complete unless checks pass, or a clearly identified non-blocking limitation is documented. Never commit known broken functionality as a completed task. Do not conceal failing tests.
7. Security tests must be confined to local fixtures or owned environments; no unauthorized attack, jailbreak of external services or unsafe destructive testing.
8. Never expose API keys in commits, logs, screenshots, fixtures or backups. Use fake credentials in tests. No telemetry or cloud accounts.
9. Verify version-sensitive Claude Code settings and provider protocols using official documentation or reproducible tests. No invented API compatibility or context capabilities.
10. Use safe, narrowly scoped config changes, atomic writes, backups and rollback. Do not change live Claude Code settings during tests; use disposable fixtures.
11. Review licenses, dependency provenance and bridge release integrity. Any bridge must have explicit install consent and loopback-only binding. No arbitrary shell execution from user input.
12. Commit and PUSH after each completed implementation/documentation task to the authorized repository; record actual hash and push confirmation. If remote is missing or push fails, STOP and ask for resolution—do not claim success.
13. After each task, give a concise progress report, but DO NOT WAIT for user approval before the next ordinary task.
14. Retry failing tasks up to three focused fix-and-retest cycles. If still failing, stop with a precise blocker; never silently skip critical acceptance criteria.
15. STOP for: unapproved material architecture/scope changes; new paid service; production credentials; OS elevated permissions; destructive actions; unresolved high/critical security findings; inability to verify bridge license/integrity; missing repo access; repeated failed tests. Ask only the specific needed decision.
16. If work is interrupted or context is lost, inspect `TASK_PROGRESS.md` and git history; resume from next unfinished task, never infer prior success.
17. At end, run all relevant checks, clean install/uninstall tests where feasible, and write `FINAL_AUDIT.md` with evidence, limitations and release verdict. Do not call it production-ready without real macOS smoke testing.
18. Windows remains future scope; cross-platform abstractions are required but Windows functionality cannot be claimed without Windows tests.
