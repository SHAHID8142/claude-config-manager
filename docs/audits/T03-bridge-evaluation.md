# T03 — Evaluate open-source bridges

**Date:** 2026-11-08 · **Result:** ✅ PASS · **Artifact:** `docs/research/bridge-evaluation.md`

## Evaluation performed
- 7 candidates assessed against the binding gates: license verified, maintenance, release artifacts, OS support, loopback binding, protocol coverage (Anthropic→OpenAI translation, streaming, tools, vision, reasoning).
- Repo metadata (license, pushed_at, releases, archived state) pulled from GitHub API — not README claims.
- Key architectural finding: OpenRouter, DeepSeek and Z.AI now ship official Anthropic-compatible endpoints → **direct mode**, no bridge. Bridge confined to OpenAI-only providers (NVIDIA NIM, custom).

## Candidates eliminated (documented reasons)
- y-router: archived, serverless-only. 1rgs/claude-bridge: deleted. maxnowack/anthropic-proxy: no license. claude-code-proxy: no releases, 8-mo stale. claude-code-mux: stale, out-of-scope failover. litellm: license NOASSERTION + heavyweight Python runtime.

## Sole surviving candidate
- **musistudio/claude-code-router** (MIT, active, prebuilt macOS/Windows artifacts, loopback default) — scope-risk recorded (v3 control-plane breadth, affiliate links, open security alert), mitigations deferred to ADR (T04) and hard tasks T49–T52.

## Security review
- No downloads performed in this task (metadata + docs only). No secrets involved.

## Tests
- Documentation-only; evidence = cited sources + API metadata in the artifact.

## Acceptance criteria
- [x] Translation, streaming, tools, vision, reasoning, license, releases, OS support, maintenance all assessed
- [x] Dead ends documented with reasons
- [x] Decision input prepared for T04
