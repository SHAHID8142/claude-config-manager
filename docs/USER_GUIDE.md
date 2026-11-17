# Claude Config Manager — user guide

Updated for 1.0.8, 2026-11-18. This app edits Claude Code's global user settings.
Project settings, managed settings and terminal environment variables can take
precedence. Restart Claude Code after changing authentication: helper output may
be cached. [Official settings documentation](https://code.claude.com/docs/en/settings).

## Set up automatic switching

1. Open **Providers & keys** and choose a provider. For a custom gateway, enter
   its name and documented Anthropic-compatible endpoint first.
2. Under **Stored keys**, give each key a label and add it. The app displays only
   its last four characters after saving. Add all keys for that provider.
3. Choose a model. Fetch a catalog where supported, or type the exact model ID
   from that provider's documentation. A catalog listing does not prove that
   the model supports every Claude Code feature.
4. Press **Add provider to switching order**. Repeat for other providers.
5. Open **Automatic switching**. Move providers up/down into your preferred
   order. Expand a provider's keys to change their priority or enabled state.
   Each provider needs its own model and endpoint. Under **Model order**, use
   **Load provider models** for suggestions or enter an exact model ID. Use
   **Add model** to add up to4 fallback models. Arrows change priority; moving
   a fallback to the top makes it primary. Remove deletes a choice from this
   order, not from the provider catalog. Each field has an information button.
6. Review the cards and press **Save & enable automatic switching**. This saves
   the visible policy, starts the local helper and updates Claude Code settings.

Example: Provider A model1 (work key → personal key) → Provider A model2
(usable keys) → Provider B model1. Adding a second chosen model from Providers
& keys appends it to the existing provider card. A healthy preferred choice stays
in use. The next provider is used after usable model/key combinations fail or
are paused, within the attempt/time limits below. Keys may share the same account quota; adding another key
from that account does not create more credit or bypass provider limits.

**Save order** only saves policy; it does not switch Claude Code from direct
mode. Once routed mode is active, saved policy changes affect subsequent
requests. **Check keys** uses provider-supported authentication/catalog checks,
not paid model inference. Unsupported checks are explained rather than guessed.

A provider's first enabled key is preferred, not necessarily the key serving a
request. Recent helper activity shows the last response key, failures and
remaining cooldowns. Saved switching history retains the last 100 failure and fallback
events across helper restarts; the screen shows the newest 20. It contains only
timestamps, provider IDs and fixed outcome categories. No prompt/response bodies,
endpoints, API keys or key labels are recorded.

### Failure behavior

- 401/403: authentication/access failure; try the next key, pause that key across models for six hours. A structured model-access error instead pauses that model.
- 402 or quota wording in 429: credit/quota failure; six-hour cooldown.
- Other 429: rate limit; Retry-After seconds or HTTP-date, capped at ten minutes, or 30 seconds.
- 5xx: provider error; Retry-After seconds or HTTP-date or 20 seconds.
- Network/timeout: ten seconds. An upstream may already have processed a timed
  out request, so switching cannot guarantee protection against duplicate charges.
- A bounded structured error explicitly identifying a missing/unavailable model
  (recognized model code, or a typed error saying the model is unavailable/not
  found) pauses that model across its keys for five minutes and tries the next
  model. A bare404 or unstructured body is insufficient evidence.
- Other invalid caller requests/endpoint errors such as400/404/422 stop the chain.
  Correct the configuration or request; these are not exhausted keys.

A request makes at most200 upstream attempts; skipped paused combinations do not consume attempts, stopping new attempts after a 60-second
retry-decision budget. Native/compatible providers retain their 30-second attempt
limit. NVIDIA uses bounded connect/send/header waits and allows a response body
to stream for up to 30 minutes, with a 120-second idle read limit; a phase already
in progress can extend past the retry-decision budget. At most 32 client connections are handled
concurrently. No replay occurs after an upstream response starts streaming.
Cooldowns are in memory. Restart the helper to retry immediately after fixing
an issue; this clears every cooldown. Manual restarts clear pauses. When a newer app activates or saves a multi-model policy, it upgrades a verified older helper first, retaining the existing port/token.
Native/compatible caller error bodies and supported protocol headers pass through. NVIDIA caller errors remain redacted.
There is no reliable automatic inference of shared account limits.

Existing policies with one explicitly selected key per target keep their order.
The **Use all enabled keys** action converts a card explicitly. Deliberately
interleaved legacy cards are never silently regrouped.

## Use a single direct connection

Choose a provider and model, add a stored key, then use **Save & Apply**. The
collapsed **Single-key direct connection (optional)** area also accepts a key
for direct Apply. This selects one static credential; automatic switching needs
the separate switching activation above. Promoting a key does not rewrite a
previously applied direct helper. Apply the new selection first. The app refuses
to disable/delete the credential referenced by an active owned direct helper.

### Replace a key that cannot be deleted

An active direct key is protected because saved settings still reference it.
You do not need a second provider account. Add the correct token with a new
**Key label** under **New API key**, select the provider's model, then use
**Save & Apply**. After the saved confirmation, remove the old entry. Adding a
key alone does not redirect an existing direct helper. For NVIDIA, Apply
activates the built-in local connection first. The optional direct-key input
and **Add key** are separate controls; Add requires both label and token.

Blank context and role fields preserve saved values. Use their explicit Reset
buttons to remove overrides. Review old model/role settings when moving between
providers: names are not portable. Manual model entry stays editable after a
catalog fetch. Context overrides depend on model support and CLI version; see
[official model configuration](https://code.claude.com/docs/en/model-config).

NVIDIA NIM now uses a built-in connection inside the local helper. Choose NVIDIA,
add its keys, select a chat model (for example the model ID published by NVIDIA),
and press Save & Apply. NVIDIA becomes the first provider group; other providers
remain as fallbacks. No separate bridge installation is needed. The helper must
run while Claude Code uses NVIDIA, including when only one NVIDIA key is stored.
The key-check button is unavailable because model listing is public and does not
prove your key works. Other OpenAI-only endpoints and the optional third-party
bridge remain unavailable; its install/start/apply commands stay disabled.
A custom endpoint must speak Anthropic Messages, not just OpenAI Chat Completions.

### Conversation instruction compatibility

The adapter accepts text `system` entries inside Messages history. NVIDIA needs
one initial system message, so active instruction text is collected there in
original order, with the top-level instructions first. `clear_at: never` persists;
`next_user_message` instructions expire when any later user/tool-result turn exists.
This translation does not preserve Claude-native cache positions or thinking
signatures. Dynamic tool additions/removals and per-message effort changes remain
unsupported and return explicit errors rather than being silently discarded.

## Recover an invalid settings file

The message “settings file contains invalid JSON — not overwritten” means the
app could not parse the file safely. Common causes include an empty/truncated
file, comments, trailing commas or a top-level array. It does not establish
which program wrote the invalid file.

The recovery banner offers **Restore valid backup** or **Start clean**. Choose a
validated backup from the selector, inspect its timestamp/model/destination/helper
availability, then confirm preserve/recover. Invalid backups remain visible with
reasons and cannot be selected. Clean recovery uses `{}`. The original invalid bytes are archived
as `invalid-settings-*.json` and excluded from ordinary backup history. Recovery
does not validate a provider's API connection. Valid settings are not replaced
through this action; use ordinary backup restore instead.

Under **Recovery & diagnostics**, **Restore selected backup** uses the chosen
validated snapshot after a second confirmation. It first preserves the current file; invalid
current bytes are kept in the separate invalid namespace. Backups are in
`~/Library/Application Support/ClaudeConfigManager/backups/`, not beside
`settings.json`. Existing valid files are backed up before Apply. First-run
creation has no previous file to back up.

## Where data lives

| Data | Default path |
| --- | --- |
| Claude Code settings | `~/.claude/settings.json` (or configured CLI directory) |
| Runtime/config data | `~/.claude-config-manager/` |
| Secrets | Runtime data `secrets/*.secret`, private mode 0600 |
| Credential labels/order/masks | Runtime data `credentials.json` |
| Helpers | Runtime data `helpers/*.sh`, executable mode 0700 |
| Provider policy | Runtime data `routing.json` |
| Switching history | Runtime data `activity/history.json` (last 100 events) |
| Interrupted-save journal | Runtime data `apply-journal.json` (private key copies) |
| Backups | `~/Library/Application Support/ClaudeConfigManager/backups/` |

Secret files are plaintext protected by filesystem permissions, **not encrypted
or stored in Keychain**. Other processes running as your user may read them.
Settings contain a helper path, not the API key. Settings backups can contain
secrets that were already present in the original settings. Treat backups as
private. JSON is parsed and pretty-printed: unrelated values are preserved
semantically, but whitespace and key formatting are not byte-preserved.

Valid model/role/context drafts are retained per provider on this device and
restored when switching providers or reopening the app. Completed custom endpoint
and model drafts can be reopened from **Saved custom provider drafts**. API key
inputs are never persisted in draft storage; invalid/incomplete values remain
session-only. Clearing app WebKit data removes these drafts. Draft storage is capped at 32 entries and 64 KiB of text; oldest drafts are
evicted first and the most recently edited draft is retained. Drafts do not activate
settings. The **Review changes** panel distinguishes pending and saved values.

Apply validates inputs/settings before managed side effects, takes backups,
checks concurrent changes, installs atomically and reads back. Handled failures
attempt rollback; this is not a multi-file crash-atomic transaction. Disk or
permission failures can also prevent rollback. An interrupted operation leaves
`~/.claude-config-manager/apply-journal.json`; further writes are blocked. The
recovery banner lets you explicitly restore previous managed files or keep current
files. Restore first archives current bytes privately. The journal and
`recovery-before-*.json` snapshots contain private key copies (base64 is not
encryption); never share them. Settings recovery is separate from provider testing.
Preserve backups before manual repair. Never paste secrets into bug reports.

## Helper and desktop use

The local authenticated helper listens on loopback only and serves
`POST /v1/messages`. Closing the GUI does not intentionally stop it. Stopping
it while settings point to it makes Claude Code unavailable until restarted or
reconfigured. Login start is opt-in; logout/reboot acceptance remains unverified
for this release candidate. The optional `/v1/messages/count_tokens` endpoint
is not implemented. See [gateway requirements](https://code.claude.com/docs/en/llm-gateway-protocol).

Section and field **i** buttons open help on focus/hover or click/tap; Escape or
an outside click dismisses it. Important errors remain inline. Use Refresh after
external file edits; the app also refreshes on window focus. Overview reports
saved global settings, not a full evaluation of all Claude Code settings scopes.

The local Apple Silicon DMG is ad-hoc signed, not Developer ID signed/notarized.
Signature integrity and Apple trust acceptance are different checks. Do not
interpret a locally valid signature as Gatekeeper approval. No security settings
are changed automatically. See the release evidence for what was actually tested.


## NVIDIA connection limits

The built-in adapter translates text, images where supported by the selected
model, function tool declarations/calls/results, usage and streamed tool argument
fragments. Tool execution remains with Claude Code, not the helper. Anthropic
server/computer tools, document blocks and structured-output formats are rejected
with a clear request error. Anthropic cache metadata is not NVIDIA cache state;
signed Anthropic thinking is not forwarded as NVIDIA reasoning. Nemotron 3 uses
`enable_thinking: false` and `force_nonempty_content: true`. NVIDIA output is capped
at its documented 32,768 tokens; a large context declaration does not increase
that output limit. Interrupted/malformed streams are not replayed.

Save and tool execution were tested end to end with the installed CLI using
loopback fixtures and fake keys. A minimal real NVIDIA request through the final
1.0.7 packaged helper returned200 with inline system text and nonempty output.
This does not certify all models, live tools, account quotas or live failover. See [1.0.7 release evidence](releases/1.0.8.md).


### Model-order compatibility and rollback

The primary model stays in the existing `model` field; optional `fallbackModels`
stores up to4 alternatives. Old single-model policies need no migration. Older
app versions reject this new field: before downgrading, remove fallback choices
and save using1.0.8, or restore a known compatible policy backup. Save & Apply in
NVIDIA setup retains the other chosen models. Each choice still needs to accept
this conversation's content and function tools; catalog presence is not a live
compatibility check. Recent activity shows the last selected model, not an
independent measurement of upstream alias resolution. Model fallback was tested
with socket/CLI fixtures; no real multi-model inference/failover was performed.
