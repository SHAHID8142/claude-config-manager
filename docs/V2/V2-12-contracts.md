# V2-12a / V2-12b — model switching and the real context window

## V2-12a: changing the model on an already-configured provider

Reported by the owner: after adding a provider and key, picking a different
model looked like it required re-entering the key.

Cause was UI-only. `save_apply::apply_with_roles` already accepts
`api_key = None` and keeps the stored credential, but the React gate was
`apiKey.trim() !== ""`, so **Save & Apply** stayed disabled.

Fix:

* new `has_credential(provider_id)` command (validates the provider id before
  touching disk, uses the same `FileSecretStore` the apply path writes);
* the gate is now `apiKey.trim() !== "" || storedCredential`, where
  `storedCredential` is only true for the **currently selected** provider
  (a stale answer from a previous selection can never enable Save);
* the key field says "A key is already stored for this provider — leave this
  blank to keep it and change only the model."

Contract: blank key + stored credential = keep the credential; blank key + no
stored credential = Save stays disabled and the backend still answers
`NoCredential` if a caller bypasses the UI.

## V2-12b: a 1M model shown as 200k inside Claude Code

### What Claude Code actually does

Claude Code sizes a session from the **model id**, not from the provider's
`context_length`. Any id it cannot match to a Claude model — every OpenRouter,
NVIDIA, or gateway spelling such as `nvidia/nemotron-…` or `my-gateway-opus` —
is assumed to be **200k**, and auto-compaction happens at that assumed window.
It prints `Model "…" isn't recognized · context assumed to be 200k tokens`.

Documented remedies
(`code.claude.com/docs/en/model-config#correct-the-window-for-a-gateway-or-custom-model-id`):

| Case | Setting |
| --- | --- |
| Unknown provider/custom id | `CLAUDE_CODE_MAX_CONTEXT_TOKENS=<tokens>` |
| Id of a model Claude Code knows | `modelOverrides` / `modelPicker.behavesAs` |
| Model with 1M | append `[1m]` to the id |
| Skip the assumption entirely | `CLAUDE_CODE_DISABLE_UNKNOWN_MODEL_WINDOW_ENFORCEMENT=1` |

`CLAUDE_CODE_AUTO_COMPACT_WINDOW` is **not** a fix: it only moves the
compaction point and is clamped to the model's window, so it cannot raise a
200k assumption to 1M.

### What the app now writes

Advanced → **Context window (tokens)** writes
`env.CLAUDE_CODE_MAX_CONTEXT_TOKENS` through the same merge pipeline as the
other managed keys:

* plain digits only; the UI strips non-digits and the merge layer rejects
  anything that is not an integer in `1_000 ..= 10_000_000` (so `1M` can never
  be written as a silently ignored string);
* blank + an effective value present ⇒ `reset_context_window` removes the key;
* blank + nothing effective ⇒ the key is left untouched, never invented;
* `read_status` exposes `env_max_context_tokens`, and the field shows
  "Effective now: N tokens" so the panel can never imply a window that is not
  actually in `settings.json`.

### Verification

* Rust 250 pass (new merge tests: write, reject `1M`/`999`/`20000000`,
  reset-keeps-neighbours); clippy 0 warnings.
* TS 154 pass (digits-only field, apply payload carries
  `maxContextTokens: 1000000`, blank key + stored credential enables Save,
  blank key + no credential keeps it disabled); eslint clean.
