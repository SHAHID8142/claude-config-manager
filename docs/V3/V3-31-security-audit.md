# V3-31 / V3-32 — Security audit: secrets, tokens, proxy surface

## V3-31 — Secret and token handling

| Concern | Finding | Evidence |
| --- | --- | --- |
| Where keys live | `~/.claude-config-manager/secrets/<credential-id>.secret`, dir `0700`, file `0600`, created `0600` (never chmod-after-write), atomic temp + `fsync` + rename, symlink refused, control characters and >8 KiB values rejected | `file_secret_store.rs:110+`, `credential_vault.rs` |
| What settings.json holds | an `apiKeyHelper` **path** only; tests assert neither the key nor a key shape appears in settings or the helper script | `tests/secret_and_network_surface.rs::settings_json_never_contains_the_credential`, `::helper_script_never_contains_the_api_key` |
| Helper script safety | path derived from a validated id (`[a-z0-9-]{1,64}`, no whitespace by construction), `$HOME` expanded by the shell, `stat -f %Lp` = 600 checked before `cat`, missing file → non-zero exit | `credential_handoff.rs:64-84` |
| Proxy bearer token | **256 bits from the OS CSPRNG** (`getrandom`), hex; generated per helper start, persisted `0600`, never returned to the UI (removed from `ProxyInfo`), never in logs, compared in constant time | `proxy.rs::random_token`, `::ct_eq`, `helper.rs::ensure_token` |
| Token file | `secrets/proxy-helper.secret`, `0600`, same store rules as provider keys | `helper.rs` unit test `token_is_persistent_and_hex` |
| Helper state | `proxy-helper.json`, `0600`, symlink refused | `helper.rs::write_state` |
| Login agent | writes one plist under `~/Library/LaunchAgents`, `Umask 077` so launchd's own log is owner-only, `gui/<uid>` domain (no admin), removable in-app | `helper.rs::plist_body` + test |
| Process signalling | the app only signals a pid that answers `/healthz` with that exact pid — a recycled pid is never killed | `helper.rs::stop`, `tests/helper_lifecycle.rs::stop_never_signals_a_pid_it_cannot_verify` |
| Cross-provider credential use | **was** possible via a hand-edited `routing.json`; now refused (provider must match, credential must be enabled) | `proxy::resolve_secret`, `tests/v3_regressions.rs` 2 tests |
| Error messages | redaction layer strips exact and shape-matched secrets; discovery errors never echo the key; model/provider ids cannot smuggle secret-shaped text | `redact.rs`, `tests/secret_and_network_surface.rs` |
| Migration | legacy per-provider secret files are read but never rewritten or deleted; no key is copied into a new location without the owner acting | `credential_migration.rs`, `docs/V2/V2-19-migration.md` |
| Logs | the proxy and helper write no request bodies, no headers, no tokens; the helper's only output is launchd's own empty log file | `proxy.rs` (no logging calls), `helper.rs::run_helper` |

### Residual risks (stated, not hidden)

1. The proxy token is stored on disk like any other credential. A process running
   **as the owner** can read it. That is the same trust boundary as `~/.claude`
   itself; the token's job is to stop other local users and accidental exposure,
   not a compromised account.
2. The proxy is HTTP over loopback, not TLS. Traffic never leaves `127.0.0.1`,
   and provider traffic upstream is HTTPS with the system trust store.
3. The helper reads provider keys from the owner's store at request time, so a
   process with the owner's file access could read them regardless of the proxy.
4. Cooldowns are in memory: a helper restart forgets them. This can mean one
   extra attempt on a target that was cooling — never a charge loop, because a
   request is attempted at most once per target.

## V3-32 — Proxy attack surface (`tests/proxy_security.rs`, 8 tests)

| Test | Assertion |
| --- | --- |
| `a_request_without_the_token_is_401_and_never_reaches_the_upstream` | no token → 401, upstream untouched, secret never echoed |
| `a_wrong_token_is_401_and_an_empty_token_is_401` | wrong token, wrong scheme, missing header → 401 |
| `a_malformed_request_line_is_400` | garbage request line → 400, listener survives |
| `oversized_headers_are_431` | 2 000 headers → 431 (64 KiB cap) |
| `a_body_larger_than_the_limit_is_413_before_it_is_buffered` | `content-length` over 16 MiB → 413 before reading the body |
| `only_post_v1_messages_is_proxied` | `/`, `/v1/models`, `/admin`, and `/v1/messages/../etc/passwd` → 404 (path matched exactly) |
| `a_crash_in_one_request_does_not_take_the_listener_down` | truncated body then valid request → the second still answers 200 |
| `the_health_endpoint_never_contains_a_credential_or_a_token` | `/healthz` returns counters only |

Additional properties from `tests/proxy_loopback.rs`: loopback-only bind, constant
time token compare, no secret in the all-targets-failed message, and no upstream
contact when the token is wrong.

**Fixed during this audit:** `/v1/messages/../x` used to pass the `starts_with`
check and produce an opaque 503. The path is now matched exactly, so it is a
clean 404 and never reaches the upstream URL builder.

## Open items

- No fuzzing of the HTTP parser (not done; no claim of robustness beyond the
  eight cases above).
- No TLS between the GUI and the helper on loopback (not planned; the token and
  loopback bind are the control).

## V3-33 — Configuration safety

| Property | Evidence |
| --- | --- |
| Backup before every write | `config_backup::create_backup` runs inside the apply lock; backup files are byte-exact and `0600` (`backup_is_byte_exact_with_0600`) |
| Atomic write + verify | temp file `0600` → `fsync` → rename → re-read and verify; a verification failure restores the original bytes exactly (`failed_verification_restores_the_original_bytes_exactly`, `atomic_write_round_trips_with_0600`) |
| Symlink refusal | `symlink_target_refused`: a symlink where `settings.json` belongs is rejected, never followed |
| Rollback | `rollback_restores_original_bytes`; the UI exposes Restore last backup |
| One apply at a time | `apply_lock` serialises applies per home; the UI cannot start a second write |
| Existing Claude Code settings preserved | `config_merge` only touches the managed keys and their explicit `clear_env_*` flags; unrelated keys, unknown keys and key order are preserved (merge property tests in `tests/merge_properties.rs`) |
| Legacy credentials untouched | migration reads legacy files and writes new ids; nothing is deleted (`credential_migration.rs`) |
| Crash recovery | `tests/crash_recovery.rs` (9 tests) covers a crash mid-apply leaving either the old or the new file, never a truncated one |
