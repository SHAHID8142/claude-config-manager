# V2-21 — credential management UI

Commands added (all validate ids before touching disk, all return metadata only):

| Command | Returns |
| --- | --- |
| `list_credentials(provider_id)` | `CredentialView[]` (id, label, last4, priority, enabled, legacy) |
| `add_credential(provider_id, label, secret)` | the new `CredentialView` |
| `remove_credential(id)` / `set_credential_enabled(id, enabled)` / `promote_credential(id)` | `bool` |

The key never travels back to the UI: the panel renders `•••• 1f2a`
(`maskKey`), and a migrated pre-V2 key shows `••••` with an `existing` badge and
**no delete button**, because it is the file Claude Code already reads.

## Why the helper had to change too

`apiKeyHelper` used to be generated for `secrets/<provider>.secret`. With several
keys per provider that file may not exist, and the helper would fail closed at
request time — which looks like "the key stopped working".

* `credential_handoff::generate_helper_script_for(provider_id, account)` now
  names the secret file after the **credential id**, both validated
  (`[a-z0-9-]{1,64}`), so the helper can only read inside `secrets/`.
* `save_and_apply` takes an optional `credential_id`. The command checks the id
  exists in the vault *before* writing anything, so a helper can never point at
  a missing file.
* The helper file itself stays `helpers/<provider>.sh`, so the `apiKeyHelper`
  path in `settings.json` (and the space-free requirement from G16) is unchanged.
* The UI sends the first **enabled** credential as `credentialId`.

## Testing note (fixed a real stale assertion)

`tests/secret_and_network_surface.rs::helper_script_never_contains_the_api_key`
still expected the pre-G16 path `ClaudeConfigManager/secrets/...` and had been
failing silently since `98b0f1c`. It now asserts the shipped
`.claude-config-manager/secrets/<account>.secret`, which is what the template
actually emits.

## Verification

* Rust 270 pass, clippy 0 warnings (vault + helper-for-account + command
  validation).
* TS 169 pass (masked rows, legacy row not deletable, toggle/promote/remove call
  the right handler, add sends label+key and clears fields, Save & Apply carries
  `credentialId`), tsc + eslint clean.
