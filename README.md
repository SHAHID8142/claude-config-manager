# Claude Config Manager

A macOS desktop utility for Claude Code global settings, provider credentials
and automatic provider/key switching. Built with Tauri 2, Rust and React.

## Version 1.0.6 — unsigned prerelease

[Download the Apple Silicon prerelease](https://github.com/SHAHID8142/claude-config-manager/releases/tag/v1.0.6) · [SHA256SUMS](https://github.com/SHAHID8142/claude-config-manager/releases/download/v1.0.6/SHA256SUMS)

Not notarized: macOS Gatekeeper may block it. See the release notes for acceptance limits.

The repair adds built-in NVIDIA activation, selected-backup previews, interrupted-save recovery, durable drafts/history and provider cards with ordered
keys, per-provider models, safer credential resolution and accessible information
buttons. The interface has four screens: Overview, Providers & keys, Automatic
switching, and Recovery & diagnostics.

**Set up:** choose a provider → add its keys → choose its model → add it to
switching. Repeat for another provider, order the cards, then **Save & enable
automatic switching**. Requests try A1 → A2 → B1 in that order. Failed keys cool
down; all-cooling keys are skipped. Caller/model errors stop the chain. Streaming
responses are never replayed. Timeouts still carry upstream-processing and
charging uncertainty.

Existing explicit-key policies retain their ordering; conversion to all enabled
provider keys is explicit. Single-key direct Apply remains available.

Read the [user guide](docs/USER_GUIDE.md), [audit and complete todo register](docs/audits/2026-11-17-project-review.md),
[release notes](docs/releases/1.0.6.md), and [limitations](docs/V3/LIMITATIONS.md).
Old screenshots/recordings in `docs/assets` demonstrate earlier versions, not
acceptance evidence for the current interface. Historical V2/V3 audits describe
their own checkpoints; the dated repair audit supersedes conflicting claims.

## Safety and compatibility

- The app modifies one global settings file. Managed/project settings and shell
  environment variables may override it. [Claude Code settings](https://code.claude.com/docs/en/settings).
- Invalid JSON is refused until explicit recovery. Invalid originals are archived
  separately; valid current settings are backed up before Apply/Restore.
- Atomic settings installation, conflict detection and read-back verification
  protect individual writes. Handled failures attempt managed-file rollback;
  a private before-image journal makes interrupted operations recoverable. Operations are not crash-atomic and rollback can itself fail.
- Secret files use mode 0600; helper scripts use 0700. Storage is plaintext under
  the current user's permissions, not Keychain or encryption. Backups are private.
- Unrelated settings values are preserved semantically; JSON formatting changes.
- Anthropic Messages-compatible providers are supported. Presets describe
  documented interfaces, not certification of every model or account. Catalog/key
  checks do not verify model inference. Context behavior depends on CLI/model.
- NVIDIA NIM uses a built-in Chat Completions adapter inside the local helper.
  Save & Apply activates NVIDIA, with key failover and function tool streaming.
  Other OpenAI-only providers and the bundled third-party bridge are unavailable.
  Bridge install/start/apply commands refuse execution.
- The local helper is authenticated and bound to loopback. It forwards supported
  Anthropic request/response headers; only `/v1/messages` is proxied.

## Local build

Requirements: current supported Node/npm, Rust toolchain, Xcode Command Line
Tools and Tauri's [macOS prerequisites](https://v2.tauri.app/start/prerequisites/).
Use the committed locks for reproducible dependency resolution.

```sh
npm ci
npm run check:frontend
npm run check:rust
npm run build:macos
```

Development browser mode uses an in-memory fixture transport; it does not write
real settings or contact providers. `?fail=env|catalog|models|apply` injects errors.

Artifacts appear under `src-tauri/target/release/bundle/`. Version 1.0.6 is a
**unsigned prerelease** (ad-hoc signature only). It is ad-hoc signed without
Developer ID/notarization. Clean-Mac Gatekeeper acceptance, VoiceOver, reboot and
real paid-provider end-to-end behavior must not be assumed. See release evidence.
No installed application is overwritten by building the candidate.

## Structure

- `src/`: interface, shared help/forms, IPC types and UI tests.
- `src-tauri/src/`: settings, vault, proxy, helper and command handlers.
- `src-tauri/tests/`: isolated-home integration, protocol, security and crash tests.
- `docs/`: current guide, architecture, research and dated audits.
- `.sps/`: approved repair plan, review coverage, evidence and unresolved work.
