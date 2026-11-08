# T07 — Acceptance fixtures, test strategy, security checklist

**Date:** 2026-11-08 · **Result:** ✅ PASS · **Artifact:** `docs/testing/TEST-STRATEGY.md` (includes the Security Checklist section)

## Defined
- Golden rule: live config never touched by tests; explicit-path design enables it (from T06 boundaries).
- Fixture plan for config/discovery/bridge/secret domains incl. adversarial variants.
- Six test layers with per-task gates; per-task acceptance mapping; CI shape (realized in T10).
- Security checklist with 25+ items across 8 domains, tied to threat-model IDs (T1–T18), to be run per task.

## Security review
- Checklist itself is the enforcement vehicle; fixture secrets are fake-keyed by convention (`sk-test-DO-NOT-USE-*`).

## Tests
- Documentation-only.

## Acceptance criteria
- [x] Fixtures specified incl. negative/adversarial variants
- [x] Test strategy mapped to backlog tasks
- [x] Security checklist executable per task
