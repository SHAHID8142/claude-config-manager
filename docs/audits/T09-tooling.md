# T09 — Formatting, lint, TS/Rust checks, test runners

**Date:** 2026-11-08 · **Result:** ✅ PASS

## Implementation
- **Frontend:** ESLint 9 flat config (`eslint.config.js`: @eslint/js recommended + typescript-eslint + react-hooks flat + react-refresh/vite); Prettier (`.prettierrc.json`, `.prettierignore` excluding dist/target/gen/lockfiles/**.md — docs keep authored formatting); Vitest + jsdom + Testing Library (`vitest.config.ts`, `src/test/setup.ts`).
- **Rust:** `cargo fmt` normalization, `cargo clippy -- -D warnings`, `cargo test` in gate script.
- **Scripts:** `lint`, `format`, `format:check`, `typecheck`, `test`, `check:frontend`, `check:rust`, `check:all`.
- **Dependency hygiene:** ESLint 10 chain pulled a moderate advisory (minimatch GHSA-g3hv-75wg-m249, DoS via glob amplification, dev-only). Resolved with npm override pinning patched `minimatch@^11.0.1` → `npm audit` now reports **0 vulnerabilities**. (Initially T05+T06-style process note: Prettier reformatted docs `.md` on first run — excluded via `.prettierignore` and files restored from git before any commit.)
- Test seeds added: `src/App.test.tsx` (render smoke), `src-tauri/src/lib.rs` unit tests incl. negative case (empty input → no panic, no newlines).

## Verification (evidence)
- `npm run check:frontend` → lint ✓, format:check ✓, tsc --noEmit ✓, vitest 1/1 ✓ (exit 0)
- `npm run check:rust` → fmt --check ✓, clippy -D warnings ✓, cargo test 2/2 ✓ (exit 0)
- `npm audit` → 0 vulnerabilities

## Security review
- Dev-toolchain only; no runtime deps added. Advisory remediation documented above (no silent suppression). Lint includes `no-explicit-any` (recommended set) — type-safety gate for T11 contracts.

## Tests
- Runners proven by seeds; subsequent tasks must extend suites (rule: every new module ships ≥1 positive + ≥1 negative test).

## Acceptance criteria
- [x] Formatting, lint, TS checks, Rust checks, unit test runners configured and green
- [x] Negative test seed present in Rust suite
- [x] Zero npm audit findings
