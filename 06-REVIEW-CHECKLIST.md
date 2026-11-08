# Independent GitHub Review Checklist

After each task user shares repo URL + branch + commit hash + agent report.
Reviewer verifies actual committed diff, not agent assertions.
- Scope matches exactly one approved task; no unrelated edits.
- Build/lint/typecheck/test evidence is reproducible.
- New code has unit/negative tests; unsafe cases covered.
- Configuration merge preserves unrelated keys; rollback and file permissions tested.
- Secrets absent from tracked files, logs, fixtures and artifacts.
- Provider API/Claude Code compatibility backed by official docs or real tests.
- Bridge license, version pin, download integrity, localhost binding and lifecycle tested.
- UI accessibility and error states verified.
- No regressions; no unsupported claims about native model capabilities.
- Risk register updated; release blocked by unresolved high/critical findings.
- Verdict: APPROVE / REQUEST CHANGES / BLOCKED, with exact next single-task prompt.

Final 'ready to use' requires macOS install, real VS Code CLI session, direct and bridged provider tests, backup/restore test, security review and clean uninstall. GitHub code review alone cannot prove real-world runtime readiness.

## Autonomous handoff review
The independent ChatGPT review is requested ONCE after the coding agent completes the backlog, not after each task. Examine every commit and `TASK_PROGRESS.md`/`FINAL_AUDIT.md`; check that tasks were atomic, tests and push evidence exist, and no stop conditions were ignored. Return PASS / NEEDS FIXES / BLOCKED with concrete file-level findings and remediation prompt(s).
