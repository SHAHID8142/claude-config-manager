# V2-03 — Claude Code / router compatibility (verified against sources)

## A. Claude Code constraints (authoritative, previously verified in this repo)

| Fact | Source | Consequence for V2 |
| --- | --- | --- |
| Exactly **one** `apiKeyHelper` path, executed unquoted | `docs/audits/G16-space-in-helper-path.md` (observed in a live terminal) | one active credential per applied config; a space is fatal |
| Exactly **one** `ANTHROPIC_BASE_URL` | `docs/audits/T02*`, `config_merge.rs` | failover must happen behind that one URL |
| Precedence: `ANTHROPIC_AUTH_TOKEN`/`ANTHROPIC_API_KEY` (env **or** shell export) > `apiKeyHelper` | T02 + G15/G15a | any failover credential must stay in a file the helper reads, never an env var we don't control |
| `ANTHROPIC_MODEL` > `settings.model` | T02 + G15 | the router must be addressed through `settings.model` or a single base URL, not a second env model |
| No documented fallback/rotation feature | ADR-002 | rotation can only be implemented behind a URL we control |

## B. claude-code-router (musistudio) — checked 2026-11-08

| Property | Value (verified) |
| --- | --- |
| License / activity | **MIT**, 38 042 stars, last push 2026-10-10, not archived |
| Latest release | **v3.1.3** (2026-10-09) |
| Distribution | Electron desktop app (`dmg` **120 MB**, `zip` 119 MB, Windows `exe`, Linux `AppImage`) + npm CLI `@musistudio/claude-code-router` (Node ≥ 22) |
| Gateway / UI ports | gateway `http://127.0.0.1:3456`, management UI `http://127.0.0.1:3458` |
| Features relevant to V2 | credential pools, key rotation, ordered fallback models, conditions, retries, per-request logs (`packages/core/src/providers/credential-pool.ts`, `packages/ui/src/pages/home/shared/fallbacks.ts`) |
| **Configuration format** | **SQLite** `~/.claude-code-router/config.sqlite` (0700/0600). Docs (`configuration-file.md`): *"A legacy `config.json` is read only once as a migration source when no SQLite config exists; after migration, editing `config.json` does not affect the current configuration."* Writes should go through the management RPC with a `configRevision` CAS token (409 on conflict). |

### Why the old plan (generate `config.json` from our app) is dead
The v1/v2 JSON config described in our earlier notes no longer applies. There is **no documented file-based
configuration path** in v3. Programmatically configuring CCR would mean either writing someone else's SQLite
database while their service runs (explicitly discouraged by their docs) or speaking their private
revision-token RPC — neither is a contract we can test honestly.

## C. Decision (material — see ADR-003)

1. **Ship our own minimal loopback failover proxy in Rust** (`127.0.0.1`, ephemeral port, token-protected),
   because it is the only way to satisfy the Phase E requirements *verifiably*: we must classify errors,
   bound retries, apply cooldowns, and prove no duplicate tool execution. ~600 lines, no third-party binary,
   no new trust root, no consent gate, and it reuses the credential store we already audited.
2. **Also detect and support a user-run CCR gateway** (health probe on `:3456`, explicit "external gateway"
   mode) rather than trying to configure it. Detection only; never write their DB.
3. **Never** silently substitute a model: every routed response carries the model that actually answered, and
   the UI/log surface shows it (ADR-002 preserved).

Rationale for not shipping CCR as the mandated path: 120 MB binary + Electron tray we do not control, config
we cannot write safely, and a trust/consent surface that outweighs the ~600 lines we need. The prompt's
"prefer an existing verified gateway" is honoured by the *external gateway* mode above; the built-in proxy is
the fallback for users who do not run one.
