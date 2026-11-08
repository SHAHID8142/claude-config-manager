# Open-Source Bridge Evaluation (T03)

**Research date:** 2026-11-08 · **Status:** COMPLETE — evidence-based comparison for T04 ADR
**Question:** which open-source component can translate Anthropic Messages protocol → OpenAI-compatible backends (streaming, tools, vision, reasoning) as a local, loopback-only, license-clean, maintained helper?

## 0. First finding: several target providers no longer need a bridge

Verified against official provider documentation (each to be re-verified when its preset ships in T24):

| Provider | Protocol for Claude Code | Evidence | Mode |
|---|---|---|---|
| Anthropic | native Anthropic Messages | official docs | **DIRECT** |
| OpenRouter | Anthropic Messages compat: `POST https://openrouter.ai/api/v1/messages`; Claude Code guide: `ANTHROPIC_BASE_URL=https://openrouter.ai/api/v1` + `ANTHROPIC_AUTH_TOKEN`; supports text, images, PDFs, tools, extended thinking, streaming | openrouter.ai/docs/api-reference/messages; openrouter.ai/docs/cookbook/coding-agents/claude-code-integration | **DIRECT** |
| DeepSeek | Anthropic compat: `ANTHROPIC_BASE_URL=https://api.deepseek.com/anthropic`; streaming SSE, client tool use supported; x-api-key + Bearer both accepted | api-docs.deepseek.com/guides/anthropic_api | **DIRECT** |
| Z.AI | Anthropic compat: `ANTHROPIC_BASE_URL=https://api.z.ai/api/anthropic` + `ANTHROPIC_AUTH_TOKEN` | docs.z.ai/devpack/tool/claude | **DIRECT** |
| NVIDIA NIM | OpenAI-compatible only (`integrate.api.nvidia.com/v1`) — no Anthropic endpoint | NVIDIA NIM docs | **BRIDGE** |
| Custom | user-provided endpoint, protocol unknown | — | **BRIDGE** (if not Anthropic-compatible) |

Consequence: the bridge is required **only** for OpenAI-only providers (NVIDIA NIM, custom OpenAI endpoints). This matches PRD F09/F10 (direct connections require no bridge; bridge is on-demand).

## 1. Candidate matrix

| Criterion | musistudio/claude-code-router (CCR) v3.1.3 | BerriAI/litellm v1.110.x | luohy15/y-router | 1rgs/claude-bridge | fuergaosi233/claude-code-proxy | maxnowack/anthropic-proxy | 9j/claude-code-mux |
|---|---|---|---|---|---|---|---|
| Purpose | local gateway translating agent protocols (Anthropic Messages ↔ OpenAI Chat/Responses, Gemini…) to many providers | general LLM gateway, Anthropic-format passthrough + translation | Anthropic→OpenRouter translation as Cloudflare **Worker** | deleted from GitHub | Anthropic→OpenAI proxy | Anthropic→OpenAI translation | Anthropic multiplexer w/ failover |
| License | **MIT** ✓ | NOASSERTION (mixed MIT + enterprise dirs) ✗ | MIT ✓ | n/a | MIT ✓ | **none declared** ✗ | MIT ✓ |
| Maintenance | active (pushed 2026-10-10; release v3.1.3 2026-10-09) ✓ | very active ✓ | **archived** (2026-01-11), no releases ✗ | **404/deleted** ✗ | last push 2026-03-12 (~8 mo), **no tagged releases** ✗ | stale 2025-04 ✗ | stale ~1 yr ✗ |
| Releases/artifacts | prebuilt macOS arm64/x64 DMG, Windows exe, Linux AppImage; npm dist ✓ | PyPI/npm/Docker; heavy ✓✗ | none ✗ | — | none ✗ | none | none |
| OS support | macOS/Windows/Linux (npm + prebuilt) ✓ | any (Python/Docker) but heavyweight | Cloudflare only ✗ | — | Node/Bun, unverified packaging | Node | Rust, stale |
| Loopback default | **127.0.0.1:3456** (gateway) + 127.0.0.1:3458 (mgmt UI) ✓ | host-configurable | serverless ✗ | — | configurable | configurable | configurable |
| Streaming/tools/vision/reasoning | streaming, tool-call translation, image content, thinking handling implied + documented in full docs; observability records tool calls/images ✓ (deep verification in T55/T56 with fixtures) | full feature parity (large project) ✓ | streaming/tools only ✗ | — | partial, unverified | partial | n/a |
| Fit for a *minimal controlled helper* | ⚠ v3 is a broad "control plane" (mgmt UI, credential pools, affiliate links, 952 open issues, 1 open security alert in repo Security tab) — much larger than our need; we can confine usage to its gateway core with pinned version, minimal config, mgmt UI not surfaced | ⚠ full gateway needing Python/Docker runtime — poor fit for a small pinned helper | ✗ | ✗ | ✗ maintenance | ✗ license | ✗ stale, failover out of scope |

## 2. Dead-end notes (why candidates fail the security contract)

- **y-router:** archived + serverless (Cloudflare Worker) — violates loopback-only local-helper model and maintenance gate.
- **1rgs/claude-bridge:** repository deleted — supply-chain dead end.
- **maxnowack/anthropic-proxy:** no license → unusable.
- **claude-code-proxy (fuergaosi233):** viable features but no tagged releases, 8 months stale → cannot meet version-pin + integrity + maintenance gates.
- **claude-code-mux:** stale; failover features explicitly out of PRD scope.

## 3. Conclusion (feeds T04 ADR)

Exactly one candidate passes the mandatory gates (verified license, active maintenance, prebuilt platform artifacts, loopback default, protocol coverage): **musistudio/claude-code-router**. Its v3 scope exceeds our minimal-helper ideal; T04's ADR must record mitigations (pin exact version + checksums; drive only its gateway via config; never expose its management UI as our UX; disable where possible; integrity verification before execution per T49–T52).

Fallback decision path if CCR fails deeper verification (T49 checksums / T55 fixtures): STOP per backlog T04 rule ("if no safe candidate exists, STOP for decision") and present options to the user.

## Sources

- https://github.com/musistudio/claude-code-router (README; repo metadata via GitHub API 2026-11-08)
- https://github.com/BerriAI/litellm, https://github.com/luohy15/y-router, https://github.com/fuergaosi233/claude-code-proxy, https://github.com/maxnowack/anthropic-proxy, https://github.com/9j/claude-code-mux (metadata via GitHub API)
- Provider docs: openrouter.ai/docs/api-reference/messages · openrouter.ai/docs/cookbook/coding-agents/claude-code-integration · api-docs.deepseek.com/guides/anthropic_api · docs.z.ai/devpack/tool/claude
