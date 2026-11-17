# Current module boundaries — 1.0.5

Reviewed 2026-11-17. This is an implementation map, superseding the original v1
CLIAdapter/Keychain design sketch. No CLIAdapter abstraction or KeychainStore is
implemented in this repository.

| Boundary | Implementation and responsibility |
| --- | --- |
| Interface | React App/tabs/components; input, draft state, masked credentials and help |
| IPC | `src/lib/ipc.ts` ↔ async-dispatched blocking Tauri commands in `lib.rs` |
| Settings | `config_locator`, `config_reader`, `config_merge`, `config_writer`, `config_status` |
| Backup/recovery | `config_backup`, `config_restore`; bounded selected-backup previews and confirmed restoration |
| Managed operations | `transaction_journal`, `apply_lock`, `save_apply`, `nvidia_setup` |
| Credentials | `credential_index`, `credential_migration`, `credential_vault`, `credential_service`, `file_secret_store` |
| CLI handoff | `credential_handoff`; generated private shell scripts and quoted executable commands |
| Provider metadata | `provider_catalog`, `presets`; embedded protocol/auth/catalog declarations |
| Network checks | `model_discovery`, `model_discovery_service`, `connection_test`; bounded HTTPS, redirects disabled |
| Routing | `routing_config`, `failover`, `proxy`; ordered provider groups expanded to enabled keys |
| NVIDIA | `nvidia_adapter`; Messages ↔ hosted Chat Completions translation, bounded JSON/SSE/tools |
| Detached lifecycle | `helper`; same executable in `--proxy-helper` mode, PID/health identity and opt-in LaunchAgent |
| Activity | `switch_history`; 100 private redacted events; UI displays newest 20 |
| Dormant bridge | `bridge/*`; historical pinned third-party integration; execution commands refuse unavailable feature |

The UI does not read private files or make provider inference requests directly.
Rust validates IPC inputs independently. Blocking command bodies are dispatched
away from the main thread. Most domain modules can be tested without the GUI.

A managed Apply writes a private before-image journal under a shared cross-process
lock. Planned new key/helper files are tracked before creation. Success removes
the marker; handled failure compensates owned files and reports rollback failures.
Settings writes have independent backup, atomic installation, conflict checks and
read-back compensation. An external settings edit is preserved on failed Apply.
After interruption, new managed operations refuse until explicit restore/keep
recovery. This is recoverability, not a crash-atomic filesystem transaction.

Credentials are private plaintext files, not encrypted or in Keychain. Journals
and recovery snapshots can contain key copies. No private journal bytes cross IPC.
Backup previews expose only selected metadata, never source JSON or key values.

Provider drafts use bounded, validated local WebKit storage independently of
`routing.json` and global CLI settings. Secret input is omitted. Custom endpoints
with userinfo/query/fragment are not saved as drafts. Draft restoration never
activates configuration. Switching activation saves the visible policy and
settings under one journal rather than two independent IPC writes.

The helper authenticates loopback traffic, replaces upstream credentials and
forwards supported protocol headers. Provider groups preserve key priority;
explicit legacy targets keep their order. Failure history excludes prompts,
response bodies, endpoint URLs, key values and user-supplied key labels. Cooldown
state is temporary; account-wide quota sharing cannot be inferred reliably.

See [the guide](../USER_GUIDE.md), [security model](../security/THREAT-MODEL.md),
and [audit evidence](../audits/2026-11-17-project-review.md). Test evidence is
bounded to the cases actually executed, not certification of every provider.
