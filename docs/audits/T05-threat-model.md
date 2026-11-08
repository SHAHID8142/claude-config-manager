# T05 — Threat model

**Date:** 2026-11-08 · **Result:** ✅ PASS · **Artifact:** `docs/security/THREAT-MODEL.md`

## Coverage
- Assets (5), trust boundaries (6), threats (18) ranked by severity across all contract domains: credentials (T1/T2/T15/T17), config corruption & races (T3/T4/T5), localhost (T9), supply chain (T7/T8), OS integration (T14/T18), protocol fidelity (T10), endpoints/metadata (T11/T12/T13), IPC (T16).
- Every threat mapped to concrete backlog tasks; residual risks stated explicitly (no silent acceptance).

## Security review
- Threat model itself contains no secrets, no live endpoints beyond documented public ones, no attack recipes (defensive document only).

## Tests
- Documentation-only. Cross-checked against `02-ARCHITECTURE-SECURITY.md` threat list — all named threats covered (T1–T13 map directly; OS integration added as T14/T18).

## Acceptance criteria
- [x] Credentials, config corruption, localhost exposure, download supply chain, OS integration all modeled
- [x] Severity ranking + mitigation-to-task mapping
- [x] Residual risk disclosure
