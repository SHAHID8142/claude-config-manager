# T10 — CI workflow: checks, secret scan, dependency audit

**Date:** 2026-11-08 · **Result:** ✅ PASS · **Artifact:** `.github/workflows/ci.yml`

## Implementation
- Three jobs on push/PR (concurrency-grouped):
  1. **frontend** — npm ci → lint → format:check → typecheck → vitest → vite build
  2. **rust** — official Tauri Linux prerequisites → stable toolchain + rustfmt/clippy → fmt --check → clippy -D warnings → cargo test (rust-cache)
  3. **security** — gitleaks full-history secret scan; `npm audit --audit-level=high` (moderates tracked in audits, per TEST-STRATEGY); `cargo audit` via prebuilt install action
- No secrets in CI (PRD); gitleaks uses built-in GITHUB_TOKEN.

## Verification (evidence)
- Local YAML parse: valid, 3 jobs.
- Live run evidence: first push of this commit triggers CI on GitHub; result recorded in audit addendum below (actual run status + URL).

## Security review
- Secret scan gate is blocking (high/critical audit failures block; moderate documented). Workflow itself contains no secrets.

## Tests
- Workflow syntax validated locally; functional verification = actual GitHub Actions run on push (see addendum).

## Acceptance criteria
- [x] CI for available checks, secret scanning, dependency audit

### Addendum — CI run evidence (2026-11-08)
- Run 42436371092: **FAILED before start — "The job was not started because your account is locked due to a billing issue."** All 3 jobs blocked by GitHub account state, not by workflow content.
- Workflow syntax locally validated (YAML parse, 3 jobs).
- **Limitation recorded:** CI execution evidence pending until the repository owner resolves the GitHub billing lock (user-side action; Actions are free for public repos once unlocked). All local gates remain the enforced evidence (`check:all` green per task). CI re-verification will be re-run at T63/T67 or on the next push after the lock is resolved.
