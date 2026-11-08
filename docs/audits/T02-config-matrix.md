# T02 — Verify Claude Code global configuration; document matrix

**Date:** 2026-11-08 · **Result:** ✅ PASS · **Artifact:** `docs/research/claude-code-config-matrix.md`

## What was verified (official sources only)
- **Paths & precedence:** managed > `--settings` > project-local > shared project > user (`~/.claude/settings.json` — our only write target). `~/.claude.json` never written. Strict-JSON requirement. `CLAUDE_CONFIG_DIR` honored on read.
- **Auth precedence:** Bedrock/Vertex/Foundry > `ANTHROPIC_AUTH_TOKEN` > `ANTHROPIC_API_KEY` > `apiKeyHelper` > `CLAUDE_CODE_OAUTH_TOKEN` > profile/federation > subscription OAuth. Storage: macOS Keychain (fallback `.credentials.json` 0600).
- **`apiKeyHelper`:** shell script returning key, hot-reload, >10s warning, blocked under managed `forceLoginMethod`.
- **Env vars:** full auth/routing/model matrix captured incl. `ANTHROPIC_DEFAULT_*_MODEL` role aliases and empty-string-as-unset semantics.
- **`env` block semantics:** no shell expansion (absolute paths), settings win over shell, set-not-remove.

## Negative/adversarial findings that shape later tasks
- Local machine already routes via a pre-existing proxy (`127.0.0.1:31415`) with auth env set → app must detect pre-existing env values and warn before overwriting (feeds T34).
- Plaintext tradeoff for `ANTHROPIC_API_KEY`/`AUTH_TOKEN` in settings.json explicitly documented → T32 decision point with consent, STOP condition recorded.
- `ANTHROPIC_BASE_URL` ≠ api.anthropic.com disables Remote Control → disclosure requirement (feeds T19/T54).

## Security review
- No secrets recorded anywhere in the document (local values redacted before writing). Cross-check was strictly read-only.

## Tests
- Documentation-only task; verification = source URLs + redaction applied. `git diff --stat` clean of unrelated changes.

## Acceptance criteria
- [x] Config paths, keys, precedence, auth mechanisms documented as a matrix
- [x] Official sources cited; unverified/version-specific keys marked as non-write-set
- [x] Security implications routed to T32/T34 backlog items
