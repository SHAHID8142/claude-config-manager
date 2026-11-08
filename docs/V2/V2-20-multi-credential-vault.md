# V2-20 — multi-credential vault (backend)

`credential_vault::CredentialVault` is now the only writer of
`credentials.json` and the only mapping from credential id → secret file.

Layout:

```
~/.claude-config-manager/
  credentials.json            # metadata, 0600, atomic replace
  secrets/openrouter-1.secret # one 0600 file per credential
  secrets/openrouter.secret   # pre-V2 file, never moved or rewritten
```

Guarantees:

* **write order** — secret first, index second. If the index write fails the
  secret is deleted again, so there is never a record without a key.
* **atomic replace** — the index is validated, written to a temp file (0600),
  `fsync`ed, then renamed over the original.
* **idempotent delete** — `remove` returns `false` for an unknown id and leaves
  the disk untouched; deleting a known id removes secret and record.
* **legacy alias** — a migrated `<provider>-legacy` record resolves to the
  existing `secrets/<provider>.secret`, so upgrading neither copies nor renames
  key material. `add` then appends the new key *after* it in the chain.
* **read is pure** — `load` migrates legacy entries in memory only; nothing is
  written until a mutation explicitly saves.
* **ids** — generated as `<provider>-<n>` (`n` = first free slot), validated by
  the same `[a-z0-9-]{1,64}` rule used for file names, so a vault call can never
  address a path outside `secrets/`.
* **enable/disable** — `set_enabled(false)` keeps the credential (and its
  secret) but removes it from the active chain returned by
  `active_for_provider`.
* **promote** — `promote` renumbers only that provider's chain.

Verification: 7 vault tests (multi-add/list/resolve/masked metadata/0600 index,
persistence + chain order, delete + idempotency, enable/disable, legacy
upgrade alias, unsafe-id/blank refusals, RFC 3339 timestamp shape).
`cargo test` 269 pass, clippy 0 warnings.
