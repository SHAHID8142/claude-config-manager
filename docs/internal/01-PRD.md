# Product Requirements Document — Claude Config Manager
Version 0.1 | Status: proposed for user approval

## Product objective
A lightweight single-window desktop utility to configure Claude Code CLI without remembering shell commands. Primary user runs Claude Code in VS Code's integrated terminal. macOS release first; architecture supports Windows. Not a replacement for Claude Desktop.

## Confirmed decisions
- Tauri 2 + Rust + React + TypeScript + Tailwind CSS.
- Minimal single window, no sidebar or embedded terminal.
- Global Claude Code CLI configuration only; no per-project editing.
- Provider templates: Anthropic, OpenRouter, NVIDIA NIM, Z.AI, DeepSeek and Custom; only publish verified working templates.
- Enter API credentials and endpoint; fetch model catalog when supported, searchable, NEVER auto-select. Manual model ID fallback.
- Save & Apply, status, validation, test connection, backup/restore.
- Provider/model native defaults remain unchanged unless a supported explicit override is requested.
- Optional on-demand open-source bridge is a MUST-HAVE for incompatible protocols. First use: request consent to download/install. App window can close while bridge remains operational when needed.
- No automatic failover, multi-agent routing, cloud sync, database, account system or project-specific profiles in v1.
- Future CLI adapters (Codex/Gemini/OpenCode) without implementing them now.

## Main screen
Active configuration summary; provider selector; endpoint field; masked API key field with reveal control; fetch/refresh models; searchable model selector; manual ID field; direct/bridge connection indicator; Test Connection; Save & Apply; collapsible Advanced; clear validation and error states.

## Functional acceptance
F01 Read existing global settings without modifying them.
F02 Preserve unrelated JSON keys and detect parse/permission errors; never overwrite invalid source.
F03 Validate URLs, required fields, supported protocol and selected model before applying.
F04 Fetch models via documented provider endpoints only; never silently select a model.
F05 If discovery unsupported/fails, allow explicit manual model ID; indicate unknown capabilities.
F06 Save only explicitly supported Claude Code settings, using correct precedence for CLI/environment variables. Detect likely override conditions and report uncertainty.
F07 Backup before writing; atomic write; restore on failure; no partial configuration.
F08 Never put raw credentials in logs, telemetry, crash reports, or unencrypted backups. Keychain alone is insufficient if CLI cannot read it: explicitly design and test the secure handoff.
F09 Direct Anthropic-compatible connections require no bridge process.
F10 Non-direct connection uses an evaluated and pinned open-source bridge, with explicit install consent, checksum/signature validation, health checks, controlled lifecycle, restart/recovery and uninstall.
F11 Closing the GUI must not break an active bridge-dependent CLI session; explain helper lifecycle and opt-out.
F12 Provider/model capabilities displayed only when verified; no invented context windows, vision/reasoning/tool support.
F13 Support dark/light system theme, keyboard navigation, accessible labels, reasonable small window size.
F14 macOS packaging and clean-install tests; Windows architecture retained, not falsely marked supported until tested.

## Out of scope
Automatic model failover; priority routing; concurrent models; project-specific config; custom proxy engine; embedded terminal; CLI tool installation; Claude Desktop configuration.

## Quality gates
Unit tests for merge/validation/backup; integration tests with disposable test config; provider and bridge smoke tests (no real secrets in CI); security tests for injection, localhost binding, log leakage, malicious download, invalid JSON, symlink and concurrent writes; accessibility review; rollback and clean install.

## Open technical decisions (must be researched before implementation)
1. Current Claude Code configuration keys, auth conventions, precedence and whether desktop integration differs.
2. Provider-specific Anthropic protocol support; whether chosen models actually support Claude Code tool use.
3. Open-source bridge candidate's repository, license, release artifacts, maintenance, supported protocols and platform packaging.
4. Secure key delivery to CLI without leaving credentials exposed; if persistent env configuration is necessary, disclose implications and obtain approval.
5. Exact on-demand helper startup trigger when user starts Claude Code externally from VS Code. A helper cannot reliably auto-start merely because the GUI is closed unless a launcher/service integration exists; document the chosen tradeoff.
