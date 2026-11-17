# Current release limitations — 1.0.8

Updated 2026-11-18; supersedes conflicting V2/V3 historical acceptance claims.

- Apple Silicon macOS only. Windows/Linux builds are not produced or tested.
- Ad-hoc signature only. Developer ID/notarization and clean-Mac Gatekeeper
  acceptance are unavailable without the owner's signing identity. No automatic
  security bypass is provided.
- NVIDIA is supported through a built-in Chat Completions adapter. Other
  OpenAI-only providers and the optional third-party bridge are unavailable. Bridge
  install/start/apply commands refuse execution; dormant pinned dependencies
  include three moderate advisory records in their separate lock audit.
- Claude Code uses `/v1/messages`; NVIDIA targets translate to Chat Completions.
  Token counting is optional
  in the upstream gateway specification and is not implemented here.
- Secret storage is private plaintext files, not Keychain/encryption. Same-user
  processes can read them. Existing plaintext in settings may be in backups.
- A private before-image journal supports explicit interrupted-save recovery.
  Journal/recovery snapshots contain key copies protected by permissions, not encryption.
  Individual settings writes are atomic; all managed files are not one
  crash-atomic transaction. Rollback is attempted on handled failures and may
  itself fail under disk/permission errors. A tiny final check/rename race with
  non-cooperating external editors remains possible for existing settings files.
- Legacy interleaved explicit-key routing targets retain their order. Conversion
  to provider groups is explicit. Custom profiles persist when added to routing;
  valid setup drafts persist separately in local WebKit storage; clearing that
  storage removes drafts but does not delete stored keys.
- Keys can share account quotas. The proxy classifies observed HTTP responses;
  it cannot reliably determine key-level versus account-wide health. Caller/model
  errors stop failover. Retry-After seconds and HTTP-date are supported and
  bounded to ten minutes. Cooldowns reset on helper restart and are not persisted.
- Retry decisions stop after 200 candidates/60 seconds. Native/compatible
  attempts have a 30-second maximum. NVIDIA phase waits are bounded and may
  finish after the decision budget; its stream can last up to 30 minutes with
  120-second idle reads,
  with 32 concurrent connections. No retry after response streaming starts.
  A timeout may leave upstream processing uncertain; duplicate charges cannot
  be ruled out. Saved history retains 100 redacted failure/fallback events, not complete request logs.
- Catalog/key checks do not verify model inference. OpenRouter key check returned200. A later minimal NVIDIA inference through the1.0.7 packaged helper returned200 with nonempty text. This does not certify every model/tool or live failover. The installed CLI completed a
  streamed response through the packaged helper using loopback fixtures only.
- Overview reads one global scope. Project/managed settings and another
  terminal's environment may override it; helper output can be cached by CLI.
- Blank roles/context preserve saved values. Review model and role compatibility
  when switching providers; saved role IDs are not automatically translated.
- Native small-window/200% zoom, VoiceOver, contrast tooling, real logout/login,
  reboot, clean-Mac installation and remote CI are not certified. See release
  evidence for successful checks and concrete native automation limitations.

The full outstanding register is in
[the repair audit](../audits/2026-11-17-project-review.md). An unchecked item is
not silently accepted as complete because automated tests pass.

1.0.7 accepts text system messages inside conversation history, collecting active
instructions into NVIDIA's initial system message. Turn expiry is respected;
cache placement is not portable. Dynamic tool changes/per-message effort remain
unsupported. Basic bare/default-CLI fixtures use the actual saved NVIDIA model,
not a forced Claude model; this is broader coverage, not every CLI feature.


Source CI for1.0.7 passes frontend/Rust but the full dependency gate fails on
[GHSA-fc5q-39xc-vjpf](https://github.com/advisories/GHSA-fc5q-39xc-vjpf), a high
ReDoS advisory in picomatch4.0.7 used by Vite/Vitest. No upstream patched release
was available when checked. Production-only npm audit reports0; this does not
clear the development gate. Cargo audit was skipped after npm failed.


1.0.8 adds ordered model fallback with primary +4 alternatives per provider.
Provider/model/key order, scoped cooldowns, no-replay and full real-CLI tool flow
are fixture-tested. Actual hosted multi-model inference remains unverified. Older
apps reject fallbackModels; remove alternatives before downgrading. Current7 helper
is deliberately left running until the owner reopens8 and saves/enables its policy.
