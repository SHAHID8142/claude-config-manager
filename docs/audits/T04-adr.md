# T04 — Select bridge with evidence; write ADR

**Date:** 2026-11-08 · **Result:** ✅ PASS (no STOP required — safe candidate exists) · **Artifact:** `docs/adr/ADR-001-bridge-selection.md`

## Decision recorded
- Direct mode default for all Anthropic-protocol providers; bridge only for OpenAI-only providers.
- `musistudio/claude-code-router` selected, version-pinned at integration; MIT license verified via GitHub API.
- Six risks documented with task-mapped mitigations; explicit fallback = STOP-for-decision if T49/T55 verification fails.

## Why no user STOP was needed
Backlog T04 mandates STOP only "if no safe candidate exists". One candidate passes every mandatory gate (license, maintenance, artifacts, loopback, protocol coverage). Risks are recorded with mitigations rather than accepted silently.

## Security review
- Selection is documentation-only; no download or execution occurred. Integrity verification is deferred to T49 as planned.

## Tests
- Documentation-only; evidence = T03 matrix + cited sources.

## Acceptance criteria
- [x] ADR with evidence, risks, mitigations, fallback
- [x] Direct-mode-first architecture established
- [x] STOP conditions for later bridge tasks pre-registered
