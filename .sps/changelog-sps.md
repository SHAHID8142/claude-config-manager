# SPS change log

## 2026-11-17 — first contact

- Read installed SPS 4.0.0 workflow, audit and plan gate.
- Audited source paths, local settings metadata, quality checks and release artifacts.
- Wrote 75/100 baseline workflow alignment score, 30 findings, 54 remediation tasks,
  proposed repair plan, help inventory and baseline tracked-text file hashes.
- Captured user's clarified provider-first / key-priority switching requirement.
- Requested concrete plan approval. No implementation approval recorded yet.
- Changes are audit/memory documents only. No secrets stored in audit artifacts.

## Approval

User selected “Batch approve the full plan” on 2026-11-17. All five sections authorized.


## 2026-11-17 — approved implementation and local candidate

- Recovered the zero-byte settings file after preserving exact invalid bytes.
- Repaired vault aliases/resolution/ownership, migration, reset intent, private
  metadata/settings writes and handled-failure rollback guard.
- Added grouped key expansion, bounded retry/concurrency, cooldown visibility,
  protocol-header/caller-error forwarding and no-replay stream error reporting.
- Simplified to Overview, Providers & keys, Switching and Recovery; added shared
  info buttons and clear pending/saved state; made credential forms await saves.
- Updated current docs and marked V2/V3 phase snapshots historical.
- Passed 204 frontend and 345 Rust tests (five live tests ignored), lint/format/
  typecheck/clippy. Built 1.0.2 app/DMG using Finder-free packaging after Apple's
  Finder AppleEvent timeout blocked Tauri's usual image customization.
- Verified signature integrity, image integrity, read-only mount/copy equivalence
  and actual CLI through packaged helper against loopback fake-key fixture.
- Gatekeeper assessment rejected ad-hoc app; no Developer ID/notarization.
  Native minimum-window resize automation failed; zoom/VoiceOver/reboot/live
  provider and clean-Mac checks remain explicit outstanding work.


## 2026-11-17 — clarify unsupported-provider Save (1.0.3 local)

Rechecked the user's native NVIDIA draft. Settings valid/writable; activation
blocked by protocol guard. Corrected the misleading footer/bridge-setup/verified
framing, added persistent explanation plus accessible action description, and
disabled unsupported key check. 205 frontend tests pass, including no NVIDIA
write and supported-provider successful fixture Apply. Built and verified local
1.0.3 signature/image. No restart of current user draft, settings/key write or
remote publication. This does not implement hosted NVIDIA activation.


## 2026-11-17 — NVIDIA implemented, local 1.0.4

- User explicitly requested missing NVIDIA support and uninstall/cache cleanup.
- Added built-in request/JSON/SSE/function-tool translator with bounded buffers,
  existing failover, safe caller errors, interrupted-stream no-replay behavior.
- Save & Apply creates NVIDIA-first provider policy, starts helper, writes and
  verifies CLI settings; input/settings preflight and failed-policy restoration.
- Native Save enabled with stored key/model. 206 frontend,356 Rust tests pass;
  real CLI tool round trip passed with signed packaged helper, fake keys/local
  fixtures only. Image/signature checked; ad-hoc Apple trust remains rejected.
- Installed app and LoginAgent/cache/WebKit folders moved to Trash; identified
  old helper stopped. Keys/settings/backups preserved. Native test app closed
  and recreated caches cleared. No reinstall, paid call, push or public release.
