# T44b — DeepSeek catalog credential style fixed (per-discovery auth override)

**Date:** 2026-11-08 · **Result:** ✅ PASS (docs-verified + header proven on loopback) · **Artifacts:**
`provider_catalog.rs`, `model_discovery_service.rs`, `presets/deepseek.json`

## The defect (found during T44a verification)
`GET https://api.deepseek.com/models` returned **401**. DeepSeek's official docs
(`api-docs.deepseek.com/api/list-models`) specify `Authorization: Bearer <token>` for that call, while
our template declared `auth: "api_key"` — so discovery sent `x-api-key` and would have failed **for every
user, even with a valid key**. The T38 audit had live-verified only OpenRouter; DeepSeek's adapter had been
wired but never proven.

Root cause: `ProviderTemplate.auth` is a **single** field describing the *chat* credential style, but the
catalog call is a different API surface. For DeepSeek the two genuinely differ:
chat = Anthropic-compatible (`x-api-key`), catalog = OpenAI-style (`Bearer`).

## Implementation
- `ProviderTemplate.discovery_auth: Option<AuthStyle>` with `#[serde(default)]` — an explicit, optional
  per-catalog credential style. Absent means "same as `auth`", so all four other presets are unaffected
  and need no edit. Unknown values are rejected by serde (tested).
- `model_discovery_service::resolved_discovery_auth(template) = discovery_auth.unwrap_or(auth)` — one
  place decides, used by `discover_for_provider`.
- `presets/deepseek.json`: `"discovery_auth": "bearer_token"`, chat `auth` unchanged.

## Security review
- The override only selects **which header** carries the credential; it cannot add, drop or widen
  credential use. Both styles were already implemented and tested (T37).
- No credential is attached when the catalog is public: new test
  `credential_is_never_sent_when_catalog_is_public` proves a `BearerToken` template with `None`
  credential sends neither `authorization` nor `x-api-key`.
- Presets remain compile-time embedded (`include_str!`), so the override cannot be tampered with at
  runtime (T07 supply-chain discipline).

## Tests (evidence)
- `model_discovery.rs` — replaced the old single echo test with a request-capturing fixture:
  - `api_key_style_sends_x_api_key_header` (and asserts **no** `authorization:`),
  - `bearer_style_sends_authorization_header` (and asserts **no** `x-api-key:`) ← proves the DeepSeek path,
  - `credential_is_never_sent_when_catalog_is_public`.
- `model_discovery_service.rs`:
  - `deepseek_catalog_uses_bearer_not_the_chat_api_key_style` (chat stays `ApiKey`, catalog becomes
    `BearerToken`) — direct regression guard,
  - `providers_without_an_override_keep_their_chat_style` (openrouter/anthropic/nvidia-nim/zai unaffected).
- `provider_catalog.rs`: `discovery_auth_override_is_optional_and_parsed` (absent → `None`, present →
  parsed) and `discovery_auth_override_rejects_unknown_style` (negative).

Gates (local, this commit): `cargo fmt --check` ✓ · `cargo clippy -- -D warnings` ✓ ·
`cargo test` → **150 passed, 2 ignored (live network) + 7 integration = 157** ✓ ·
frontend unchanged: **98/98** ✓ · `npm run build` ✓

## Acceptance criteria
- [x] One provider's adapter verified per commit (NVIDIA NIM in T44a, DeepSeek here)
- [x] Verification grounded in the provider's official documentation, not inference
- [x] Regression tests pin the resolved style and the wire-level header
- [x] No credential sent to public catalogs

## Adapter status after T44
| Provider | Catalog | Status |
|---|---|---|
| OpenRouter | `openrouter.ai/api/v1/models` | ✅ live-verified (T38, re-run in sweep: 461 models) |
| NVIDIA NIM | `integrate.api.nvidia.com/v1/models` | ✅ live-verified (T44a: 84 models, no auth) |
| DeepSeek | `api.deepseek.com/models` | ✅ docs-verified + header proven; **not** live-verified (needs a real key) |
| Anthropic | `api.anthropic.com/v1/models` | ⚠️ docs-verified + fixture-tested; live verification requires a production credential → rule 15 STOP |
| Z.AI | none | ⚠️ no official catalog endpoint documented → manual entry (T41) |

## Known limitations
- **DeepSeek not live-verified**: proving it end-to-end requires a real DeepSeek API key. The header,
  URL, shape and error paths are proven offline; the claim stops there.
- **Anthropic not live-verified** for the same reason; it remains the recommendation to test with a
  throwaway key when one is available.
- Switching the credential style does not change what `save_and_apply` writes: DeepSeek chat continues to
  use the Anthropic-compatible `x-api-key` style (T33 behaviour).
