# Bridge — Supported Features and Known Losses (T47)

**Date:** 2026-11-13 · **Pin:** `@musistudio/claude-code-router@3.1.3` (sha512-verified, see `src-tauri/src/bridge/manifest.rs`) · **ADR:** ADR-001 (companion addendum below)

## What the bridge does for this app

Translates **Anthropic-protocol requests** from Claude Code into **OpenAI-compatible calls** for providers that do not speak Anthropic (today: NVIDIA NIM; user-defined OpenAI-only custom endpoints). Anthropic-native / Anthropic-compatible providers (Anthropic, OpenRouter, DeepSeek, Z.AI) never need it.

## Verified upstream facts (pin-time research)

- The `ccr` CLI ships in the npm package and **does not require Electron**; `ccr serve --no-open` is the documented production/headless path.
- Defaults: management UI `127.0.0.1:3458`, model gateway `127.0.0.1:3456`; `--host` defaults to `127.0.0.1` — **loopback by default** (T51 invariant matches the vendor's own default; our generated config also hard-codes it).
- State/config lives in `~/.claude-code-router` (macOS/Linux); service token auto-generated unless `CCR_WEB_AUTH_TOKEN` is set.
- License MIT (GitHub API + package metadata, verified 2026-11-08).
- Requires Node ≥ 22 (`engines` field) — our installer enforces this before download.

## What our integration guarantees

| Guarantee | Mechanism |
|---|---|
| Nothing installs/executes without explicit consent | Server-side consent marker (T48); installer and runtime both refuse without it |
| Download integrity | sha512 verified against the committed pin before extraction; mismatch ⇒ refusal + no files |
| No key material in configs | Generated config references the 0600 credential **file**; the bridge reads it per request |
| Loopback only | Config hard-codes `127.0.0.1`; read-back verification fails the write otherwise |
| Survives GUI closure (T53) | `ccr serve` spawned detached (`setsid`), pid file in app state dir; status/stop work across app restarts |
| Reversible (T58) | Versioned install dir; uninstall removes it; consent withdrawal stops the bridge first |
| Safe extraction (T49) | Symlink/hardlink entries refused; `..`/absolute paths refused; entrypoint checked post-extract |

## Known losses / limitations (disclosed, never hidden)

1. **Protocol coverage is the bridge's, not ours.** Any capability the translator drops (specific reasoning-token shapes, provider-specific tool dialects, caching headers) is a bridge behavior we cannot promise. Claude Code features that depend on Anthropic-only API fields may degrade through any translator; the honest label remains "capabilities unknown" per provider.
2. **Live translation is not exercised by CI.** T55/T56 execution requires the consented runtime (T48). Automated tests cover our config generation, extraction safety, integrity, and lifecycle — the wire-format behavior belongs to the pinned upstream.
3. **Node ≥ 22 requirement.** The install path refuses without it and says so; there is no bundled runtime.
4. **CCR's own UI is not surfaced.** We never expose the management UI as our UX; it stays loopback and unused by this app (ADR-001 mitigation).
5. **Port conflicts.** CCR auto-offsets to later ports if 3456/3458 are taken; our `bridge_apply_mode` reports the gateway port from our own status record, so a vendor-initiated offset is disclosed by re-reading status before apply.
6. **No Windows support claim.** macOS-first; `setsid` detachment is Unix-only in v1.

## ADR-001 addendum (2026-11-13)

- Pin changed from "desktop zip" to **npm tarball**: the vendor's own headless path made the 120 MB Electron bundle unnecessary; the tarball is 6.5 MiB and integrity is anchored by npm's `dist.integrity` (sha512).
- Execution policy unchanged: automated tests never run the third-party binary; the runtime consent dialog is the only place `ccr serve` is ever started.
