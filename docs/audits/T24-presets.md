# T24 — Additional provider presets, one per commit (verified only)

**Date:** 2026-11-08 · **Result:** ✅ PASS · **4 commits:** `6dc26c3` (OpenRouter), `c989338` (DeepSeek), `af66992`+`d7e0ccf` (Z.AI + repair), `ceb1780` (NVIDIA NIM)

## Presets added (all evidence-based before writing)

| Provider | Protocol | Auth | Direct base URL | Discovery | Source |
|---|---|---|---|---|---|
| OpenRouter | anthropic_compat | bearer_token | `https://openrouter.ai/api` | public `/api/v1/models` (live-verified HTTP 200, no auth, `data[]`) | openrouter.ai docs |
| DeepSeek | anthropic_compat | api_key | `https://api.deepseek.com/anthropic` | `https://api.deepseek.com/models`, auth required, `data[]` (docs-verified) | api-docs.deepseek.com |
| Z.AI | anthropic_compat | bearer_token | `https://api.z.ai/api/anthropic` | **none** — no verified catalog endpoint (honest absence) | docs.z.ai/devpack/tool/claude |
| NVIDIA NIM | open_ai_only | bearer_token | none (bridge owns upstream URL) | public `/v1/models` (live-verified HTTP 200, `object:list`+`data[]`) | docs.nvidia.com (URL verified HTTP 200) |

## Schema refinement (NIM commit)
- Per-protocol base-URL policy: native → forbidden to override; compat → required; open_ai_only → forbidden (bridge owns the upstream URL). New negative tests for all three branches.

## Process deviations (honest record)
1. Z.AI commit `af66992` shipped with two test files whose python string-replaces silently no-oped (Prettier had reflowed the target lines): fixture import missing and selector expectation stale → **frontend tests failed on the pushed commit**. Repaired in follow-up commit `d7e0ccf` (no force-push). Lesson recorded: verify every scripted edit's effect (assert in scripts), prefer the Edit tool.
2. A `presets.rs` test insertion via python initially landed outside the test module (compile error) — repaired within the same working session before any commit.

## Security review
- Every preset URL is https + doc-cited; discovery `requires_auth` flags match live probing; compile-time embedding retained. No invented capability claims; Z.AI discovery honestly absent.

## Tests (evidence, final state)
- Rust **53/53** (preset tests per provider incl. discovery shapes and bridge metadata).
- Frontend **51/51** (fixture parity, selector list incl. NVIDIA NIM, bridge-required provider present).
- clippy/fmt clean; frontend gate green.

## Acceptance criteria
- [x] Presets added individually ONLY when verified; one provider per commit
- [x] Fake/no credentials anywhere
- [x] Honest discovery absence where unverified
