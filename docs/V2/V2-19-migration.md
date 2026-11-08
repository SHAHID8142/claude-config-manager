# V2-19 — backward-compatible migration of existing keys

Legacy layout (shipped today): one credential per provider at
`~/.claude-config-manager/secrets/<provider-id>.secret`.

V2 layout: metadata in `credentials.json`, secrets at
`secrets/<credential-id>.secret`.

Migration is **derivation only**:

* `pending_legacy_providers(index, stems)` lists providers that have a legacy
  secret file and no record yet. The caller supplies a directory listing of
  file *stems*; the function never opens a secret and never reads key bytes.
* `migrate_legacy(index, stems, now)` returns a new index with one record per
  pending provider: id `<provider>-legacy`, label `Existing key`, priority 0,
  enabled, `last4` left **empty** (unknown, not guessed).
* The secret file is never moved, copied, or rewritten, so the legacy file the
  synthesized record resolves to is exactly the one the app already used.
* Re-running with the migrated index adds nothing (idempotent), and a provider
  that already has any credential is never touched.
* Stray directory entries (`""`, `../escape`, `Bad Name`, duplicates) are
  ignored. A provider id long enough to make `<provider>-legacy` exceed 64
  characters is skipped rather than truncated — a truncated id could resolve to
  the wrong file.
* Any validation failure while building the migrated index returns the original
  index, so a bad migration cannot leave the app unable to load its own state.

Verification: 5 unit tests (migrate-once/idempotent, never overwrite, stray
entries ignored, sorted+deduped pending list, over-long id skipped);
`cargo test` 262 pass, clippy 0 warnings.
