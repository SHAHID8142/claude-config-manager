# G15 — shadowed credentials ("applied and verified" while the key was ignored)
#      + a real Test Connection, and the primary action made unmissable

**Date:** 2026-11-08 · **Trigger:** owner report after adding an OpenRouter key — *"i cannot test the
connection check Configuration applied and verified."*

## Defect 1 — Save & Apply could report success while the stored key was **never used**

`save_apply` wrote `apiKeyHelper` and left `env` alone (`None` = "leave untouched" by design). So a plaintext
`ANTHROPIC_AUTH_TOKEN` / `ANTHROPIC_API_KEY` left in `settings.json` by another tool survived — and Claude
Code ranks env credentials **above** `apiKeyHelper`, so the helper was dead weight. The app said
"Configuration applied and verified" (true: the file *was* written) while the credential it had just stored
was ignored. The UI warning existed, but the app should not create or preserve that state.

The owner's real file showed exactly this: `ANTHROPIC_AUTH_TOKEN` (59 chars, plaintext) **and**
`ANTHROPIC_API_KEY` (empty string — still "present" for precedence) alongside a freshly written helper.

### Fix
`DesiredSettings.clear_env_credentials` (mirroring `clear_env_base_url`, because `None` cannot mean
"delete"): in helper mode the merge **removes** `ANTHROPIC_AUTH_TOKEN` and `ANTHROPIC_API_KEY` before any
env write, including the blank-string case. Unmanaged keys are preserved.

### Verification (against the owner's actual file, read-only)
```
env keys before=9 after=7
ANTHROPIC_AUTH_TOKEN: before_present=true  after_present=false
ANTHROPIC_API_KEY:    before_present=true  after_present=false
unmanaged preserved: CLAUDE_CODE_AUTO_COMPACT_WINDOW=true
apiKeyHelper set: true
```
Tests: `helper_mode_removes_shadowing_plaintext_credentials`,
`env_mode_still_writes_plaintext_credentials_when_asked` (env mode unaffected).

## Defect 1b — the same shadowing for the model
`ANTHROPIC_MODEL` also outranks `settings.model`, so a stale `ANTHROPIC_MODEL=auto` (found in the owner's
file) meant the model picked in the UI was ignored while the app reported success.
`DesiredSettings.clear_env_model` removes it whenever a top-level model is written
(`writing_a_model_removes_the_shadowing_env_model`). Env-model mode is unaffected.

## Defect 2 — no way to test that a key works

"Applied and verified" verifies **the file**, not the credential. The placeholder "Test Connection" button
was permanently disabled.

### New: real connection test
`src/connection_test.rs` + `test_connection` command + wired button. One authenticated request to a
**documented** key-check endpoint:

| Provider | Endpoint | Why it proves the key |
|---|---|---|
| OpenRouter | `GET https://openrouter.ai/api/v1/key` (Bearer) | requires auth (docs verified); `/models` is public and would prove nothing |
| Anthropic | `GET https://api.anthropic.com/v1/models` (`x-api-key` + `anthropic-version`) | catalog requires the key |
| DeepSeek / NVIDIA NIM | their catalog endpoints (Bearer) | catalog requires the key |
| Z.AI / Custom | — | no documented endpoint ⇒ "unsupported", never guessed (rule 9) |

Same hard policy as discovery: HTTPS, redirects disabled (a 3xx must not forward the key), proxy from the
environment, OS trust store, 10 s timeout. Results are classified honestly: 200 accepted · 401 wrong/expired
· 402 no credit · 403 key lacks permission · 404 wrong path · **429 accepted but rate limited** (ok) · 5xx
provider-side (key not judged). Error text is passed through the secret-shaped stripper.

Tests (8): 200/401/403/404/429 classification, OpenRouter target is the key endpoint (not the public
catalog), unsupported providers refused, empty credential refused before any request, and a recording
server proving the credential is sent in the **configured** header style.

UI: `Test Connection` sits in the action row with the note *"Makes one real request with this key;
Save & Apply only checks the file."* It is disabled until a provider and a key are present. 4 new UI tests
cover enabled/disabled, success, rejection (asserting "Connection OK" is absent), and a backend refusal.

## Defect 3 — the primary action was below the fold
At both 800×600 and 880×880 the API-key field was visible but **Save & Apply was not** — found from a real
screenshot, not from code. The action row is now a **fixed footer** (with `pb-28` on the page so nothing is
hidden behind it). Verified in the running bundle: all three buttons visible at the bottom of an 880×880
window.

## Gates
```
cargo test    243 passed, 0 failed    clippy -D warnings, fmt   clean
vitest        128 passed / 14 files   lint, prettier, tsc       clean
```
