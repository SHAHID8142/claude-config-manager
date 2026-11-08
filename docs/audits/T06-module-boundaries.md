# T06 — Define module boundaries

**Date:** 2026-11-08 · **Result:** ✅ PASS · **Artifact:** `docs/architecture/MODULE-BOUNDARIES.md`

## Defined
- One-directional data flow; thin Tauri commands; five domain modules with Rust-shaped trait signatures (CLIAdapter, ProviderCatalog, SecretStore, ConfigWriter, BridgeManager).
- `ConfigError` taxonomy designed for redaction-safety and task T26–T36 handling.
- Cross-cutting enforcement policies (no shell interpolation, no capability fabrication, never auto-select, redaction, source-of-truth matrices).

## Security review
- Boundaries encode the threat-model mitigations (single writer for config, sole owner for bridge lifecycle, no network in ProviderCatalog).
- No speculative abstraction: exactly one CLIAdapter implementation; future CLIs explicitly not scaffolded (PRD out of scope).

## Tests
- Documentation-only; consistency check against 02-ARCHITECTURE-SECURITY.md module list — match confirmed.

## Acceptance criteria
- [x] All five modules + CLIAdapter interface specified
- [x] Ownership and forbidden-actions per module
- [x] Documentation-only scope respected
