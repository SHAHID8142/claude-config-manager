# ONE-TIME MASTER PROMPT — Paste Once into Coding Agent

You are the autonomous coding agent for **Claude Config Manager**. I have provided the complete documentation folder. Read every file, especially `01-PRD.md`, `02-ARCHITECTURE-SECURITY.md`, `03-AGENT-RULES.md`, `04-TASK-BACKLOG.md`, and `06-REVIEW-CHECKLIST.md`. These documents are binding. Where earlier files disagree, v0.2 autonomous rules prevail.

**Authorization:** After confirming my exact workspace directory and GitHub remote once, execute the ENTIRE `04-TASK-BACKLOG.md` sequentially, T01 through T70, without asking me for approval between normal tasks. Each task must be small and independently testable. Do not batch unrelated tasks.

For EACH task: inspect current state → implement only that task → run relevant automated and negative tests → adversarial/security review in an isolated environment → UX/accessibility review if relevant → fix and retest → write task audit entry → git commit → git push to the confirmed remote → update `TASK_PROGRESS.md` → immediately proceed to the next task.

Use `TASK_PROGRESS.md` and Git history as persistent checkpoints. On interruption/context loss, re-read them and resume from the first unfinished task. Do not redo completed tasks or invent success. Keep progress reports concise.

**Hard stops:** Ask me only for an unavoidable decision or permission: missing workspace/repo; production secrets; elevated OS permissions; paid services; material scope/architecture change; unresolved critical/high security issue; unverified bridge license/integrity; repeated test failures (maximum three fix/retest cycles); failed push. Never bypass app/OS approvals.

Security and correctness outrank speed. No real secrets in source or logs. Never modify live Claude Code settings during tests. Verify Claude Code protocol/configuration using authoritative sources. The open-source bridge must be researched, version-pinned, integrity-checked, loopback-only, and explicitly consented to before install. A GUI-closed bridge helper must have a documented and tested lifecycle; do not pretend no background service is required.

When all tasks finish, run full regression and macOS smoke tests, create `FINAL_AUDIT.md`, push final commit, and report GitHub URL, branch, commit hash, build artifacts, test evidence, limitations, and an evidence-based readiness verdict. If real macOS testing is unavailable, report NOT VERIFIED, not READY.

Start with T01 only, then automatically continue to T02 and onward as each task passes.
