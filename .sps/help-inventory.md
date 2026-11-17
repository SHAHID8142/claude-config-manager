# Information buttons — implementation copy inventory

Status: shared help implemented in FieldRow, Section, HelpPopover and Tooltip;
provider group/key help, selected backup preview, interrupted-save recovery,
review changes and saved switching history are implemented.
Do not treat the inventory alone as full native accessibility acceptance.

Use a consistent `i` button next to each section/field label. Give it a specific
accessible name such as “About API key order”. Keep the icon modest but the hit
area usable. Open on hover/focus and on click/tap/Enter/Space; close on Escape
and outside interaction. Keep content reachable and readable while hovered.
Do not put interactive buttons inside labels or other buttons. Use unique IDs,
visible focus, viewport-aware placement and WebView feature-detected fallbacks.
Required instructions, errors and recovery actions remain inline.

Text below describes the **intended repaired behavior**. Do not publish text
that promises grouped switching or transaction rollback before it exists.

| Section / field | Help text |
| --- | --- |
| Overview | Shows saved global settings, pending changes and recent checks. Project, organization or terminal settings may override the global file. |
| Saved configuration | Read from the settings file shown here. Refresh to check external changes; this does not inspect a running Claude Code conversation. |
| Provider | The service that receives model requests. Choosing a provider edits your draft until you apply it. |
| Model summary | The model ID saved for this provider. The provider must accept that exact ID and support the required Messages format. |
| Key summary | Shows the active masked key or the next eligible key. Disabled or cooling keys are skipped by automatic switching. |
| Automatic switching summary | Providers are tried in order. Within each provider, enabled keys are tried in their own order. |
| Environment summary | Checks CLI availability, settings readability and possible overrides. A successful check does not prove a model request will succeed. |
| Providers & keys | Add each service once, then store its keys inside that provider. Select a model for every provider you want to use. |
| Provider preset | A documented endpoint template. A preset describes protocol support; it does not verify your balance, access or chosen model. |
| Provider name | A label used in this app and switching history. It is not sent as a model ID or API credential. |
| Endpoint URL | The provider's Anthropic-compatible base URL. Check the final Messages URL preview to avoid repeating `/v1`. |
| Protocol | Claude Code needs a compatible inference format. OpenAI-only endpoints require a working translator and are unavailable without one. |
| Provider enabled | Allows this provider to receive requests during automatic switching. Disabling it keeps its stored keys. |
| Provider order | All usable keys in the provider above are tried before moving here. Reorder providers to change your fallback preference. |
| API keys section | Keys belong to this provider. The app shows labels and masked endings; secret values remain in private local files. |
| Key label | A name such as Work or Personal that helps you identify this key. Do not paste a secret into the label. |
| API key | Paste a key issued by this provider. It is stored locally with owner-only permissions and passed through a helper, not copied into settings.json. |
| Stored key | The app displays only a masked ending after saving. Replacing a key requires entering the new value. |
| Key enabled | Allows this key to be selected. Disabling it leaves it stored but makes it ineligible for new routed requests. |
| Key order | Keys are tried top to bottom within this provider. A healthy first key stays in use; the next key is used when the earlier one is unavailable. |
| Add key | Saves this key under the current provider. Saving a key alone does not activate automatic switching. |
| Replace key | Replaces the selected credential after validation. Any active references and displayed mask are updated together. |
| Delete key | Removes the stored credential. If an active configuration uses it, choose a replacement or disable that reference first. |
| Test key | Makes a documented authentication check when available. It does not send a full coding task or prove the chosen model can answer. |
| Key health | Last observed check or request result and time. Unknown means no usable evidence; rate limited means temporarily unavailable. |
| Shared account limits | Multiple keys from one account may share the same quota or rate limit. Switching keys may not restore service. |
| Model settings | Choose the exact model ID this provider will receive. Fallback providers can use different IDs, so configure each one explicitly. |
| Fetch models | Loads the provider's documented catalog. It does not select or activate a model. Some providers require a stored key. |
| Search models | Filters the fetched list by ID or display name. Filtering never changes your current selection. |
| Model ID | The exact ID expected by this provider. You can type an ID manually even if it is missing from the catalog. |
| Catalog capabilities | Values are reported by the provider. Missing capability data remains unknown rather than being guessed. |
| Catalog context size | The provider's published token capacity. It is evidence for a setting, not proof Claude Code recognizes this ID's window. |
| Advanced settings | Optional role and context controls. Leave unchanged values alone; use an explicit Reset action to remove an override. |
| Context window | Tells Claude Code the assumed window for supported custom IDs. Recognized Claude IDs and `[1m]` IDs follow different rules; check the version-specific note shown here. |
| Adopt catalog window | Copies the selected model's published token count into your draft. Review compatibility before applying it. |
| Haiku role | The model requested for the Haiku role on this provider. A blank untouched field keeps its saved value; Reset removes it. |
| Sonnet role | The model requested for the Sonnet role on this provider. It must be an ID this provider accepts. |
| Opus role | The model requested for the Opus role on this provider. It must be an ID this provider accepts. |
| Additional roles | Role support depends on Claude Code version. Show only supported controls and preserve unrelated existing values. |
| Reset override | Removes this specific saved override after you apply. It does not delete the provider or its keys. |
| Review changes | Compares the pending configuration with saved values before writing. Secrets are always masked or omitted. |
| Apply configuration | Validates and backs up managed state, writes it and verifies the result. File verification is separate from provider connectivity. |
| Automatic switching | A local helper routes Claude Code requests. Keys are tried within the preferred provider before the next provider is used. |
| Switching enabled | Saves and activates the reviewed provider/key order through the helper. Turning this off offers a deliberate switch to a direct provider. |
| Order preview | The exact sequence eligible requests will follow, such as Provider A Key 1 → A Key 2 → Provider B Key 1. |
| Cooldown | A temporary pause after a failed key or service. The next eligible choice is used; the displayed time says when this choice can be tried again. |
| Retry unavailable keys | Explicitly requests a new check of paused choices. It does not reset provider balances or guarantee service is available. |
| All choices unavailable | No enabled choice can currently serve the request. The reason and earliest retry time help you decide whether to wait, fix a key or change a provider. |
| Last switch | Shows why the previous choice failed and which choice was tried next. History contains no key values or prompt content. |
| Check providers | Checks the saved policy's available key-check endpoints. Unsupported checks remain unknown; unsaved edits are not represented as tested. |
| Local helper | Runs the local routing service after this window closes. Claude Code cannot use automatic switching while the helper is stopped. |
| Start / Stop helper | Starts or stops local routing. Stopping it can interrupt new requests if Claude Code still points at it. |
| Restart helper | Replaces a stale helper and verifies the new process. Requests in progress may be interrupted. |
| Start at login | Installs a user LaunchAgent so automatic switching is available after login. You can remove it here. |
| Request counters | Counts observed helper requests, fallback attempts and failures since this helper session started. It is not a billing statement. |
| Bridge | Translation for OpenAI-only providers is not ready in this build. Choose a supported compatible endpoint until a verified translator is available. |
| Recovery & diagnostics | Inspect settings problems, restore backups and refresh environment checks without exposing secrets. |
| Settings file | The exact file this app edits, including a configured CLAUDE_CONFIG_DIR. Other settings scopes may override it. |
| Invalid or empty settings | The app found a file it cannot parse. Preserve it, then restore a validated backup or explicitly start a clean configuration. |
| Recovery preview | Shows the source backup, affected path, saved endpoint/model and helper availability before recovery. A valid JSON backup does not prove an API connection. |
| Backups | Settings snapshots taken before changes when a previous file exists. A first run with no settings file has nothing to back up. |
| Restore backup | Restores the selected valid settings snapshot and preserves the current bytes first. Stored keys and provider balances are separate. |
| Create clean settings | Preserves the invalid file and creates a new JSON object after confirmation. You will then configure providers and apply them. |
| Config directory | Reports whether the directory needed for settings can be written. A missing first-run directory is different from denied access. |
| Shell exports | Values set in a terminal can override parts of saved configuration. The app lists relevant variable names without displaying secret values. |
| CLI version | The installed Claude Code version. Some model/context and gateway features require newer releases. |
| Refresh diagnostics | Repeats local checks and updates their timestamps. It does not edit your terminal profile or contact a paid inference endpoint. |
| Appearance | Light, dark or system appearance for this app. It does not change Claude Code's terminal theme. |

Implementation must also cover equivalent fields in the legacy target editor
while that editor remains available: target provider, target credential, target
model, target order, target enabled, save chain, route, and health. Do not add a
separate `i` tab stop to every repeated model/key row when the shared column
label or section help explains the same field unambiguously.


1.0.5 adds explicit info/help for selected backup metadata, interrupted managed
recovery choices, saved custom drafts, review and bounded history. FieldRow now
always supplies an info affordance. Source and unit coverage are distinct from
native VoiceOver/small-window acceptance, which remains unverified.
