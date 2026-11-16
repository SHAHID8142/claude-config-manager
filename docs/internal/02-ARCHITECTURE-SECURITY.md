# Architecture and Security Contract

## Modules
UI (React/TS) → typed Tauri commands → Rust application service → ClaudeCodeAdapter + ProviderCatalog + SecretStore + ConfigWriter + BridgeManager.
Define a small CLIAdapter interface, implement ClaudeCodeAdapter only. No speculative generic plugin framework.

## Configuration safety
Locate global config using platform-aware home path and official Claude Code documentation. Read existing file; schema-aware targeted merge; preserve unrelated settings; snapshot original with restrictive permissions; write temp file in same directory; fsync where feasible; atomic rename; verify read-back; rollback if verification fails. Handle missing file, malformed JSON, symlink, concurrent edits and permission denial safely. Never modify project-level settings.

## Credentials
Prefer OS credential vault; do not persist keys in app JSON. CLI credential handoff must be verified, not assumed. If a third-party CLI requires plaintext environment values in a persistent file, present the security tradeoff before implementation; avoid unconsented secret writes. Mask UI and redact all logs/errors.

## Bridge boundary
Bridge accepts local requests only; bind loopback, not 0.0.0.0; protect local API from untrusted callers where supported. No public network listener. Pin release versions and verify integrity before execution. Verify license, provenance, supported OS/architecture, upstream maintenance and test compatibility. Ensure bridge does not silently strip tool calls, vision, streaming, reasoning or model metadata. Treat capability support as per-provider/per-model, not universal.

## Lifecycle
Direct mode: no helper. Bridge mode: GUI closure does not terminate needed helper. Design a separately managed helper with explicit user-visible status, stop/uninstall, restart policy, minimal privileges and documented startup method. Do not promise 'no background process' in bridge mode.

## Threat model
Untrusted endpoints; malicious model metadata; poisoned downloads; compromised dependency; credential exfiltration; logs and backups leaking secrets; localhost abuse; shell/argument injection; path traversal; config corruption; race conditions; misleading connection tests; provider billing and quota errors. Use no shell interpolation with user input; validate URLs/schemes; restrict executable locations; use dependency scanning and lockfiles.

## Capability policy
Default = provider/model native. Never fabricate context limits or modify model capabilities. Claude Code may impose additional limits and use compaction; disclose effective capability uncertainty. Unsupported overrides are disabled, not silently applied.
