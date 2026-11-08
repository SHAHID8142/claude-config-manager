# ADR-003 — Built-in loopback failover proxy instead of configuring a third-party router

**Date:** 2026-11-08 · **Phase:** V2 · **Status:** Accepted (supersedes the router-integration half of ADR-001)

## Context
V2 requires optional automatic failover across credentials/models/providers. Claude Code exposes a single
`ANTHROPIC_BASE_URL` and a single `apiKeyHelper`, so failover must live behind one localhost URL.

ADR-001 planned to drive `musistudio/claude-code-router`. That project's current release (v3.1.3, 2026-10-09)
stores its configuration **exclusively in SQLite** (`~/.claude-code-router/config.sqlite`); the documented
behaviour is that a legacy `config.json` is migrated **once** and then ignored. Configuration changes are meant
to go through its own UI/RPC with a `configRevision` CAS token. It also ships as a **120 MB Electron app**.

## Decision
1. Implement the failover router ourselves, in Rust, as a second binary of this project:
   - binds `127.0.0.1` only, on an ephemeral port, writes a pidfile + portfile under
     `~/.claude-config-manager/`, requires a per-install bearer token we generate (so other local users/processes
     cannot proxy through the user's paid keys);
   - reads credentials from the existing `0600` store;
   - classifies upstream errors explicitly (429 vs 402 vs 401 vs 5xx vs quota body), applies bounded retries with
     `Retry-After` respect, then fails over in priority order with cooldown;
   - never retries a request that has begun streaming; never retries non-idempotent tool-execution replays
     (the proxy is stateless — it only forwards HTTP, it never re-executes client-side actions).
2. Support **detection** of an external CCR gateway (health probe on `127.0.0.1:3456`) and let the user select it
   as the endpoint. Never write CCR's SQLite.
3. Failover stays **opt-in**; direct mode keeps zero helper processes.

## Consequences
- We own the classification table and can test it byte-for-byte (recording servers, no network).
- No new third-party trust root, no 120 MB dependency, no consent gate for binary installation.
- The external-gateway mode keeps the "use an existing router" option available without coupling to its storage.
- Cost: we must maintain the proxy; scope is bounded (Anthropic `/v1/messages` passthrough, not a translator).

## Alternatives rejected
- **Write CCR's SQLite / RPC**: undocumented, revision-CAS races, would corrupt a user's router config on schema drift.
- **Ship CCR binary and let the user configure it manually**: their UI already does this; our app would add nothing
  but a consent dialog and a 120 MB download.
