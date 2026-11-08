# ADR-001: Bridge selection — claude-code-router as pinned integration target

**Date:** 2026-11-08 · **Status:** ACCEPTED (evidence-based; fallback path defined) · **Inputs:** `docs/research/bridge-evaluation.md` (T03), config matrix (T02)

## Decision

1. **Direct mode is the default and primary path.** For every provider that officially supports the Anthropic Messages protocol, the app configures Claude Code to talk to that provider directly (`ANTHROPIC_BASE_URL` + credential env). No bridge process exists in direct mode (PRD F09). Verified official Anthropic-compat endpoints at research date: OpenRouter, DeepSeek, Z.AI; Anthropic native.
2. **Bridge is required only for OpenAI-only providers** (NVIDIA NIM, custom OpenAI-compatible endpoints).
3. **Selected bridge component: `musistudio/claude-code-router` (CCR), exact version pinned at integration time.** Current reference: v3.1.3 (2026-10-09 release). License: MIT (verified via GitHub API, not README).

## Rationale

- Only candidate passing all mandatory gates from T03's matrix: verified MIT license; active maintenance (release < 1 month old); prebuilt, checksummable platform artifacts (macOS arm64/x64 DMG, Windows exe, Linux AppImage) plus npm distribution; loopback default (`127.0.0.1:3456`); documented Anthropic↔OpenAI protocol translation incl. streaming/tools/vision/thinking paths.
- All six other candidates fail at least one mandatory gate (archived / deleted / no license / no releases+stale / mixed license+heavyweight runtime).

## Known risks and binding mitigations

| Risk | Mitigation (task) |
|---|---|
| v3 is a broad "control plane" (management UI, credential pools, affiliate integrations) — far larger than our helper ideal | We consume **only** its gateway process via a minimal, app-generated config; its UI is never surfaced in our UX; any UI-serving port stays loopback and unused (T50–T52) |
| npm distribution vs binary supply chain | Pin exact version; verify release checksums before execution; if upstream provides no checksums, compute and record SHA-256 manifest from the official release artifact at pin time and re-verify on download (T49) |
| Repo shows 1 open security alert | Dependency audit in T63 re-checks; our consumption is confined to the gateway, and network exposure is loopback-only |
| CCR config schema evolves between versions | Version pin + config schema documented in `docs/bridge/FEATURES-AND-LIMITS.md` (T47); integration tests run against the pinned version only |
| Its observability may log full request/response incl. credentials | We configure/verify minimal logging for our launch profile; secret-redaction tests (T23, T57) cover bridge diagnostics |
| Protocol translation edge cases (thinking blocks, tool_choice `any`, model-name mapping e.g. DeepSeek `claude*`→`deepseek-*`) | Capability claims stay per-provider/per-model (capability policy); T55/T56 fixture tests; known losses documented in T47 |

## Lifecycle commitments (binding on later tasks)

- Direct mode: **no helper process, ever** (F09).
- Bridge mode: helper is separately managed — explicit status UI, stop/uninstall, restart policy, loopback-only bind, GUI-closure lifecycle documented and tested (F11, T52–T53). The app never claims "no background process" in bridge mode.
- Install requires explicit first-run consent displaying source, license, exact version, artifact size and integrity info (T48).
- If at T49 the pinned artifact cannot be integrity-verified, or T55 fixture tests fail translation for our supported features, this ADR reverts to **STOP-for-decision** with the fallback options (alternate candidate re-scan or deferring bridge mode).

## Fallback

No other candidate currently passes the gates. If CCR fails deeper verification, the user decides among: (a) re-scan for new candidates, (b) ship direct-mode-only v1 with bridge marked unavailable, (c) accept documented additional risk. This is a material-scope stop per agent rules.
