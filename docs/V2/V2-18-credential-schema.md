# V2-18 — credential index schema

`~/.claude-config-manager/credentials.json` holds **metadata only**:

```json
{
  "version": 1,
  "credentials": [
    {
      "id": "work-1",
      "providerId": "openrouter",
      "label": "work key",
      "last4": "ab12",
      "priority": 0,
      "enabled": true,
      "createdAt": "2026-11-08T00:00:00Z"
    }
  ]
}
```

Rules enforced in `src-tauri/src/credential_index.rs`:

* `version` must be exactly `1`; unknown versions and unknown fields are refused
  outright (`deny_unknown_fields`), so a newer file is never half-read.
* `id` matches `[a-z0-9-]{1,64}` — it is also the secret file stem, so it has to
  be safe as a path segment and as an `apiKeyHelper` argument.
* ids are unique; labels are 1–64 characters without control characters;
  `last4` is at most four printable ASCII characters; `createdAt` is required.
* At most 200 credentials (failover chains beyond that are not a real use case).
* `for_provider` / `active_for_provider` are ordered by `priority` then `id`, so
  ordering is stable across loads and a tie cannot flip between sessions.
* `promote` renumbers only the affected provider's chain.
* `last4_of` returns an empty string for keys of four characters or fewer rather
  than revealing a short key in full.

The secret itself never enters this file: it lives in
`secrets/<credential-id>.secret` (0600) and is written by `FileSecretStore`.

Legacy compatibility (V2-19): a provider without an index entry keeps using
`secrets/<provider-id>.secret`, so upgrading cannot orphan an existing key.

Verification: 7 unit tests in the module (round-trip, version/unknown-field
refusal, unsafe id/duplicate/oversized label, ordering + disabled filtering,
promote scoping, `last4` short-secret rule, space-free path); `cargo test` 257
pass, `cargo clippy --all-targets` 0 warnings.
