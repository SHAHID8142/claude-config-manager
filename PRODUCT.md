# Product behavior — 1.0.6

Claude Config Manager edits one Claude Code global settings scope. The central
workflow is provider → ordered keys → model → switching order → explicit activation.
It serves users who want a preferred provider's available keys tried before a
backup provider, without creating a separate routing card for each key.

Overview describes saved settings and recent helper observations. Providers &
keys contains setup and optional direct Apply. Automatic switching owns provider
priority, per-provider model/endpoint, key priority and helper activation.
Recovery & diagnostics contains environment checks and backups. Each section and
field has contextual information; important failures remain visible inline.

Input is validated before writes; invalid settings require explicit recovery.
Individual settings writes are atomic and backed up when an original exists.
Handled failures compensate managed files; a private journal supports explicit recovery after interruption. Multi-file crash atomicity is not guaranteed. Provider drafts persist separately from activation; bounded redacted switching history survives restart.
Secrets remain private plaintext files under the user's account, not Keychain.

Supported protocol: Anthropic Messages over HTTP(S), including custom compatible
gateways. NVIDIA Chat Completions is translated by the built-in helper; other OpenAI-only
protocols and the optional third-party translator remain unavailable.
No catalog or key check is presented as proof of paid inference compatibility.
The app does not promise additional account quota, successful inference on every
model, no duplicate upstream charges, or full resolution of all CLI settings scopes.

See [user guide](docs/USER_GUIDE.md), [current audit](docs/audits/2026-11-17-project-review.md)
and [release evidence](docs/releases/1.0.6.md). Historical screenshots and phase
acceptance reports are not current-release verification.
