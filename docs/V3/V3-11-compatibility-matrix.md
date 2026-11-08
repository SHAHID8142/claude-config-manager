# V3-11 — Provider compatibility matrix

Source of truth: `src-tauri/presets/*.json` and `src-tauri/src/provider_catalog.rs`.
A row says **verified** only when the preset's own `capability_status` is
`verified` *and* a live request has been observed; anything else is stated as
unverified. No live provider call was made in this phase (that needs the owner's
keys and consent), so every "verified" below means "the provider's own
documentation is the source of record and the request shape matches it".

| Provider | Auth | Endpoint (base URL) | Protocol | Model discovery | Streaming | Tools | Vision | Reasoning | Context | Adapter | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Anthropic | `x-api-key` | `https://api.anthropic.com` | Anthropic native | none (curated ids) | yes | yes | yes | yes (thinking) | 200k / 1M (Claude 4.5) | none | documented, not live-verified here |
| OpenRouter | `Authorization: Bearer` | `https://openrouter.ai/api` | Anthropic compatible | `GET /api/v1/models` | yes | yes (per model) | yes (per model) | provider-dependent | per model | none | documented, not live-verified here |
| DeepSeek | `Authorization: Bearer` | `https://api.deepseek.com/anthropic` | Anthropic compatible | `GET /anthropic/v1/models` | yes | yes | no | no | 64k (chat), 128k (reasoner) | none | documented, not live-verified here |
| Z.ai (GLM) | `Authorization: Bearer` | `https://api.z.ai/api/anthropic` | Anthropic compatible | none | yes | yes | no | no | 128k | none | documented, not live-verified here |
| NVIDIA NIM | `Authorization: Bearer` | `https://integrate.api.nvidia.com/v1` | **OpenAI Chat Completions only** | `GET /v1/models` (no auth) | yes | per model | per model | per model | per model | **bridge required** | refused with `501 compatibility_error` |
| Custom | user-chosen | user-supplied | assumed Anthropic compatible | user-supplied | as upstream | as upstream | as upstream | as upstream | declared by owner | none | user's responsibility |
| Any other OpenAI-only endpoint | `Authorization: Bearer` | — | OpenAI Chat Completions | — | — | — | — | — | — | **bridge required** | refused, not translated |

## What the proxy actually does

`/v1/messages` is forwarded **verbatim** with two edits: the model id is rewritten
to the target's configured model, and the auth header is replaced with the
target's key. Nothing else is touched, so tools, images, thinking blocks and
streaming deltas pass through unchanged. Per-provider request paths are fixed at
`<baseUrl>/v1/messages`, which is why Anthropic-compatible endpoints (including
the Anthropic-compatible gateways OpenRouter, DeepSeek and Z.ai publish) work
without an adapter.

## What is refused, and how

`src-tauri/src/proxy.rs`:

```rust
pub enum Adapter { AnthropicNative, AnthropicCompatible, OpenAiUnbridged }
pub fn adapter_for(provider_id: &str) -> Option<Adapter>
```

A chain containing only unbridged targets returns, before any socket is opened:

```
HTTP 501
{ "error": { "type": "compatibility_error",
             "message": "nvidia-nim speaks the OpenAI Chat Completions API. This proxy
                          forwards the Anthropic Messages API and ships no protocol
                          bridge, so the request was not sent. Choose an
                          Anthropic-compatible provider, or install a bridge and point
                          the target at it." } }
```

In a mixed chain the unbridged target is skipped (`<provider>: needs a protocol
bridge` appears in the failure summary) and the Anthropic-compatible targets are
still tried in order. Nothing is silently translated, and no capability is
silently dropped — the failure names the missing piece.

Tests: `tests/v3_regressions.rs::openai_only_target_is_refused_with_a_bridge_message`,
`::openai_only_chain_returns_501_without_touching_the_upstream` (asserts the
upstream listener is never contacted).
