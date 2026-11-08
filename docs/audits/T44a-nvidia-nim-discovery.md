# T44a — Model discovery adapter verified: NVIDIA NIM

**Date:** 2026-11-08 · **Result:** ✅ PASS (live-verified) · **Artifact:** `model_discovery_service.rs` (live test)

Backlog T44 is "add additional model discovery adapters, one verified provider per commit". This is the
first commit of that task; it **verifies an existing adapter with live evidence** rather than adding a
speculative one — the presets were already declared in T24, but only OpenRouter had been live-tested (T38).

## Verification performed
| Check | Command / method | Result |
|---|---|---|
| Catalog reachable without credentials | `curl -s -o /dev/null -w '%{http_code}' https://integrate.api.nvidia.com/v1/models` | **200** |
| Documented shape matches our parser | `list_selector = "data"` → array of objects with string `id` | **84 entries** |
| Preset `requires_auth: false` is correct | unauthenticated GET succeeded | **correct** |
| End-to-end through our own code path | new `#[ignore]` live test `live_nvidia_nim_catalog` | **passes** (84 models) |
| Provenance label is the verified template URL (T42) | asserted in the live test | **passes** |
| Capability data still `unknown` (F12) | asserted in the live test | **passes** |

Live run: `live NVIDIA NIM catalog: 84 models, first: Some("01-ai/yi-large")`.

## Why this is worth a commit despite "no new code"
Rule 9 forbids claiming provider compatibility that has not been verified. Before this commit the NIM
adapter was *declared* but *unproven*; after it, the same claim is backed by a reproducible test that any
reviewer can re-run. The test is `#[ignore]` so CI stays network-free.

## Security review
- Public unauthenticated GET to a documented vendor endpoint; no credentials involved, nothing written.
- Provenance assertion prevents a future regression where `source` could be taken from the payload.
- Capability status remains `unknown`; NVIDIA NIM is `open_ai_only` and still requires the bridge (T47+),
  so listing models here never implies Claude Code compatibility.

## Tests (evidence)
- New `live_nvidia_nim_catalog` (`#[ignore]`, manual): passes locally, 84 models.
- Full suite unchanged and green: `cargo fmt --check` ✓ · `cargo clippy -- -D warnings` ✓ ·
  `cargo test` → **144 passed, 2 ignored (live network) + 7 integration** ✓ · frontend untouched
  (98/98 at T43).

## Acceptance criteria
- [x] One provider adapter verified with reproducible evidence
- [x] Verified against the provider's own published endpoint, not a guess
- [x] No capability claims invented

## Findings that block the remaining adapters (carried to T44b)
- **DeepSeek is currently broken.** `GET https://api.deepseek.com/models` returns **401**, and DeepSeek's
  official docs (`api-docs.deepseek.com/api/list-models`) require
  `Authorization: Bearer <token>`. Our template declares `auth: "api_key"`, so the discovery call sends
  `x-api-key` and would fail even with a valid key. Root cause: `ProviderTemplate.auth` is a **single**
  field serving both the chat credential style and the catalog call, but the two differ for DeepSeek
  (Anthropic-compatible chat vs OpenAI-style `/models`). Fix = per-discovery auth override → **T44b**.
- **Z.AI**: `https://api.z.ai/api/paas/v4/models` answers 401 (path exists) but no official documentation
  for a catalog endpoint has been verified yet → adapter stays `DiscoverySpec::None` (manual entry, T41).
  Rule 9: not claimed until documented.
- **Anthropic**: `https://api.anthropic.com/v1/models` returns 401 as expected (key required). Shape is
  documented and already consumed by T37's fixtures, but it cannot be live-verified here without a real
  credential — and rule 15 forbids using production credentials without permission.
