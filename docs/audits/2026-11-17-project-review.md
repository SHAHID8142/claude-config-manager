# Project audit and repair register — 2026-11-17

Status: baseline audit plus approved 1.0.2 remediation. The baseline below records
pre-repair observations; see the remediation evidence at the end for current state.
Application source and user settings recovery changed under full batch approval.
The installed /Applications bundle has not been replaced.
This report does not certify every line or every service as correct.

## Immediate answer: why the invalid JSON error appears

**Observed on this Mac:** `~/.claude/settings.json` is a regular file with a
size of **0 bytes**. Parsing fails at line 1, column 1, at end of input.
An empty file is not a JSON object. `config_reader::read_settings` rejects it,
and `SaveApplyService::write_desired` refuses to merge over it. That refusal
protects existing data, but the app lacks an understandable recovery workflow.

The audit did **not** establish who or what emptied the file. Do not attribute
the truncation to the app, Claude Code, or another editor without evidence.

There are 11 files matching the app backup naming convention. The five newest
parse as JSON objects. The newest is `settings-backup-1794890788644002000.json`,
407 bytes; its custom helper exists and its env contains no plaintext auth
credential. Its custom base URL ends in `/v1`, and its model is provider-specific.
Restoring it is a syntax recovery candidate, not evidence that its endpoint,
model, API key, or protocol works. Preserve the current invalid bytes first.
Do not silently replace arbitrary malformed settings with an empty object.

Claude Code 2.1.325 is installed at the normal local executable path. No relevant
ANTHROPIC or cloud-provider selection variables were present in the auditing
process's environment. This does **not** establish the environment of a VS Code
terminal or GUI-launched app; those can differ.

## What the application currently does

| Surface/module | Current behavior | Required behavior or gap |
| --- | --- | --- |
| App / tabs | One shared React draft across six screens; navigation/provider ID remembered | Keep drafts distinct from saved configuration; persist intentional provider profiles |
| Provider catalog / presets | Embedded templates for five services; explicit protocol/auth/discovery metadata | Reflect compatibility limits and unsupported operations before activation |
| Credentials | Index metadata + separate private secret files; legacy aliases derived on load | One resolution rule across all consumers; provider-scoped ordered keys |
| Model discovery | Provider catalog fetched; explicit selection; published metadata carried through | Saved-key lookup must use vault; manual override remains available |
| Connection check | Authenticated catalog/key endpoint request | Label as key check; it does not establish successful model inference |
| Direct Apply | Writes a static helper for one account, merges settings, backs up and verifies settings | Include helper/credential state in failure rollback; handle aliases and pairing |
| Status | Parses one global settings file | Report errors; describe saved settings rather than claiming complete effective CLI state |
| Backup / restore | Newest matching backup selected; current bytes backed up before restore | Validated selectable recovery candidates, shared lock, refreshed status |
| Routing | Flat ordered targets, each containing provider, key ID, optional model and base URL | Provider groups with internal key order, matching the user's requirement |
| Proxy / failover | Local authenticated Messages proxy with retry classification and target cooldowns | Key/provider health semantics, honest exhaustion, modern header forwarding |
| Helper | Same executable runs detached; explicit login LaunchAgent | Native lifecycle and upgrades must be verified in packaged release |
| Bridge | Package pin and install/runtime machinery; provider setup readiness hardcoded false | Unavailable; prevent unsupported setup paths until genuinely integrated |
| Release | Ad-hoc signed Apple Silicon 1.0.1 app/DMG | Repaired version, artifact validation and honest macOS trust instructions |

No CMS, storefront, backend account service, database server, or hosted website
was found. Local credential authentication and settings precedence are the
relevant concerns.

## Confirmed failures and inconsistencies

“Source-confirmed” below means an explicit code path establishes the issue;
it does not mean a new regression test or live-provider reproduction was run.

| ID | Priority / evidence | Failure, trigger and consequence | Required repair |
| --- | --- | --- | --- |
| F01 | P0 / machine-observed + source | Empty settings file blocks Apply; `App.tsx` swallows `readStatus` rejection, leaving status null and “Reading…” | Recovery UI and actionable read error; preserve invalid file |
| F02 | P0 / source | `save_apply.rs` maps a legacy credential alias to `store_account`, but checks existence using `account` and generates the helper using that alias. `openrouter-legacy` resolves to the real `openrouter.secret` in the vault, but direct Apply looks for `openrouter-legacy.secret` | Resolve physical account once and use it consistently; test legacy direct apply |
| F03 | P0 / source + official Tauri docs | `test_connection`, `discover_models`, `check_routing_health`, environment probes and helper operations are synchronous Tauri commands. Network/process waits can block the main thread; health checks can wait once per target | Offload bounded blocking work; test GUI responsiveness under slow calls |
| F04 | P0 / source | `apply_with_window` replaces a credential and helper before `write_desired` reads/validates settings. A malformed-file/backup/write failure can leave credential/helper changes despite reporting Apply failure. Settings backups cannot undo those side effects | Validate first; stage and rollback every managed side effect |
| F05 | P1 / source | `test_connection` in lib.rs and `discover_for_provider` use the provider-level secret file, while Add writes indexed credential files. A listed vault-only key may Apply but fail Test/Fetch; checks can instead exercise a different legacy key | Central provider/key resolver shared by all commands |
| F06 | P1 / source | Direct `save_and_apply` verifies a credential ID exists but does not verify provider ownership or enabled state; proxy resolution correctly does both | Apply must enforce the same pairing/enable checks before any write |
| F07 | P1 / source | Direct helpers read a fixed file. Promote/disable modifies only index metadata; the UI immediately says the first enabled key is “Handed to Claude Code.” An old key can still be served; deleting that file can break later requests | Distinguish pending choice from active key; atomically apply changes or resolve policy dynamically |
| F08 | P1 / source | `has_any` includes disabled records. App's availability cache is set true on Add but not refreshed on delete/disable; Apply can stay enabled with no usable key | Derive usability from enabled resolvable credentials, refresh after mutations |
| F09 | P1 / source | `contextWindow` starts blank. Every Apply sends reset=true if saved status has an override, even when the field was never edited. Provider-only changes can remove context configuration | Track dirty/keep/set/reset separately; reset only on explicit request |
| F10 | P1 / source | Provider switch clears selected model/key/context but not `roles`; merge leaves existing role overrides and model in place when omitted. A provider switch can request another provider's model/roles | Scope draft roles/models by profile; review stale saved overrides explicitly |
| F11 | P1 / source | Status effect depends on apply outcome, not restore outcome; backup/environment list refresh depends on restore, not Apply. No file watcher/refocus refresh. Dashboard's “live” claim overstates freshness | Central invalidation/refresh after successful operations and refocus |
| F12 | P1 / source | Dashboard prefers selected draft provider and credential list over configured provider; header consumes last draft key test against unrelated saved status. “Using” / “verified” can describe the wrong configuration | Bind test results to provider/key/endpoint/model fingerprint; separate saved/draft views |
| F13 | P1 / source | `canApply` allows an OpenAI-only preset; its direct URL is null and the handler has no protocol guard. Direct Apply can clear base URL and install that provider's helper even though its bridge is unavailable | Reject unsupported protocol at UI and Rust boundaries before mutating anything |
| F14 | P1 / source | Routing editor changes local state. Save chain, Check targets and Route use separate operations; health/apply handlers load disk config. Unsaved visible chain can differ from checked/applied chain; global Save & Apply may apply an unrelated direct draft while on Failover | Dirty/saved distinction; one validated save-and-enable transaction or block until saved |
| F15 | P1 / source | Current switching is per explicit target, not automatic per stored provider key. Merely adding keys never puts them all in the route | Group providers and automatically expand enabled keys in priority order |
| F16 | P1 / source | `failover::candidates` retries every cooling target when all targets cool. Dead/exhausted keys are tried again across requests instead of honoring the cooldown | Return exhaustion/retry deadline; explicit manual retry; bounded attempts/time |
| F17 | P1 / source + official gateway docs | Proxy forwards a closed three-header list and drops upstream response headers other than content type. Retry/rate-limit/feature headers and original error context can be lost | Current gateway pass-through policy with credential/hop-by-hop exclusions |
| F18 | P1 / source | CredentialsPanel invokes an unawaited void callback and immediately clears both fields. Failed Add loses entry; no pending guard. Async reload callbacks can publish results/errors for a provider that has since changed | Await outcome, preserve failed entries, disable duplicate submission, scope completions |
| F19 | P1 / source | Migration treats every valid `.secret` stem as a provider, including indexed keys and internal token accounts. Loading after adding `openrouter-1` can synthesize a phantom `openrouter-1-legacy` provider record | Restrict migration to real legacy provider/profile accounts and exclude indexed/internal accounts |
| F20 | P1 / source | Restore lacks the Apply lock. It backs up malformed current bytes under the same “settings-backup” namespace; a subsequent restore can pick the new malformed safety copy and stop | Serialize restore/apply; keep invalid recovery copies distinguishable; expose valid choices |
| F21 | P2 / source | `customName` is UI-only state; all custom endpoint keys share `custom` account; custom multi-key Add is offered but rejected. Name/endpoint are not restored as reusable profiles | Stable custom profile IDs with independent endpoint/model/key groups |
| F22 | P2 / source | Refetch clears model selection; model field becomes read-only after a nonempty catalog. Switching providers during Fetch invalidates the request but never resets fetchingModels, so the superseded request's guarded finally cannot clear the spinner. Z.AI Fetch/Test can be offered although unsupported | Reset pending state when superseding a request; preserve selection where valid; allow explicit catalog/manual mode; expose supported actions only |
| F23 | P2 / source + current model docs | Role blank labels say “default” although blank keeps old overrides; no role reset. Context guidance assumes all unknown IDs are 200K and every override works the same | Explicit keep/reset controls; current version- and model-aware context explanation |
| F24 | P2 / source | Apply feedback only renders on Dashboard. Routing errors can be hidden when routing failed to load. Several best-effort failures become permanent loading or empty states | Show errors at the action; retry affordances and distinct unavailable/empty states |
| F25 | P2 / source | Runtime helper and Loopback proxy duplicate Start/Stop controls; custom bridge panel appears on Failover; separate credentials/provider/model screens hide the user's intended sequence | Provider cards with inline keys; one runtime state and one activation path |
| F26 | P2 / source | FieldRow has no help API. Section help optional and sparse. Two help components differ; existing tooltip lacks explicit click handling, fixed positioning can clip; help targets are 16px | One accessible info-button pattern with full field/section coverage and collision handling |
| F27 | P2 / source | UI `isLoopback` uses string prefix: a hostname like `localhost.example` is misclassified and IPv6 loopback is missed. Rust routing validation parses hosts correctly | Share parsed-host semantics and test both misleading hostnames and IPv6 |
| F28 | P2 / source | UI model validator claims Rust parity but omits Rust's secret-shaped rejection. Context range validation accepts numeric forms that later fail backend integer deserialization if supplied programmatically | Align UI/backend validation and error linkage; backend stays authoritative |
| F29 | P2 / source | Dev mock returns static settings, static credentials, cross-provider fixture data and routing version 3 while backend schema is version 1; tests mostly mock IPC | Stateful contract-faithful fixtures and true command/service round trips |
| F30 | P1 / artifact-observed | DMG checksum and app signature verification pass, but `spctl` rejects the installed app. App and local release executable hashes match; ad-hoc signature is not Developer ID/notarization | Rebuild repaired release and distinguish launch trust from runtime behavior |

## Additional hardening and review debt

- Atomic settings temp files are chmodded **after** creation/write; use mode
  0600 at creation so an unusually permissive umask does not expose settings
  contents during the write. Check directory fsync/durability claims separately.
- `expected_mtime=None` skips conflict detection; first-run creation should
  detect another process creating the file before commit. Restore also snapshots
  too late to protect the entire read/backup/write operation.
- Cross-process vault mutations are protected only by an in-process mutex;
  launchable multiple app copies exist. Verify file locking and unique temp
  files, parent symlinks, oversized index/config reads, and cleanup on failures.
- Proxy spawns one thread per accepted connection; bound concurrent requests,
  total queued work, body/header sizes and total retry time. Inspect missing
  key resolution versus network failure classification and cooldown scope.
- `connection_test::interpret` treats 429 as key accepted and makes affirmative
  claims about 402/403 without response-body evidence. Show observed HTTP status
  separately from inferred credential health; rate-limited is not available.
- Shared account quotas may apply to multiple keys. Do not imply a new key
  guarantees a new quota or model entitlement. Cross-provider models need
  explicit compatible mappings; preserve requested behavior when fallback changes.
- Bridge backend commands remain registered despite a UI readiness flag.
  Review backend feature guards, PID identity before stop, prefix-based URL
  checking, consent cleanup, lockfile audit, and app/helper upgrade behavior.
- Global focus styling uses box shadow and removes outlines; verify forced
  colors fallback, focus restoration on tab changes, visible labels, target
  spacing, contrast, and overlays in the actual WebView.

## Documentation discrepancies

| Document or claim | Evidence / correction |
| --- | --- |
| USER_GUIDE calls package unsigned and un-installable | Current bundle is ad-hoc signed; Gatekeeper trust remains a separate limitation |
| USER_GUIDE says unrelated configuration is preserved byte-for-byte | serde_json reserializes the whole document; semantic values can be preserved, original bytes/formatting are not |
| USER_GUIDE says any Apply failure leaves previous configuration intact | F04 shows helper/credential side effects outside settings rollback |
| USER_GUIDE / Maintenance describe backups “next to” settings or `settings.json.backup-*` | Actual location is app support `backups/`, name `settings-backup-<digits>.json` |
| “First Save & Apply creates a backup” | Missing initial settings have nothing to back up; current writer skips backup in that case |
| Credentials copy says first enabled key is used / failover order | Direct helper remains pinned to an account until re-applied; no automatic spare-key expansion |
| “live from settings.json” and “what Claude Code is using” | No continuous refresh; only one global file parsed; project/local/managed/env/CLI overrides may win |
| “nothing is charged twice” | No replay after downstream streaming starts is useful, but timeout ambiguity cannot guarantee no upstream charge |
| “verified” preset/model | Documented protocol/catalog metadata does not establish a current successful inference with the user's key/model |
| README says discovery catalogs bind loopback | Remote provider catalogs are HTTPS clients; loopback binding describes proxy/bridge listeners |
| Old audit statuses and test counts | Retain as dated historical evidence; reconcile current release checks instead of declaring all behavior proved |

## User's required switching workflow

1. Add a provider/platform, choose a supported endpoint and model.
2. Add one or more masked keys inside that provider card. Order keys with
   simple Move up/down actions; enable or disable each key.
3. Add the next provider and its keys; provider order is independent of key order.
4. Review a readable sequence such as **OpenRouter: Work → Personal;
   DeepSeek: Primary → Spare**. A key check does not change the sequence.
5. Enable automatic switching once. The app saves a validated policy, starts
   its local helper, verifies readiness, and points Claude Code at it safely.
6. Requests try usable keys within the first provider, then the next provider.
   Show current provider/key, last failure, switch reason and cooldown time.
7. When no usable key remains, show a redacted reason and next retry time.
   Recovery controls are explicit; settings are not silently rewritten.

Model failures, malformed caller requests, transport uncertainty and already
started streams need policies distinct from a rejected key. The implementation
must test these distinctions rather than blindly retry every error.

## Complete remediation todo register

Checked means implemented with recorded local evidence, not universal certification.
Unchecked means partially addressed or outstanding, including external acceptance checks.
Priority order is stability → credential correctness → switching → usability → release.

### Recovery, state and transactions

- [x] T01 Preserve and recover the current zero-byte settings file via an explicit previewed action (F01).
- [x] T02 Distinguish missing, empty, malformed, wrong-shape, unreadable and symlink states; show Retry/Recover (F01).
- [x] T03 Add selectable validated backups and preserve invalid bytes in a separate recovery namespace (F20).
- [x] T04 Share locking/conflict detection between Apply, Restore and recovery; test cross-process races.
- [x] T05 Validate settings/input before helper or credential mutation; rollback the full managed transaction (F04).
- [x] T06 Create temporary files privately; audit fsync, first-run conflict checks, symlink parents and cleanup.
- [x] T07 Refresh status, backups, environment, credentials and helper state consistently after mutations (F11).
- [x] T08 Show operation results on the active screen and recoverable errors instead of perpetual loading (F24).
- [x] T09 Separate saved configuration, draft edits and tested identity; invalidate stale verification (F12).
- [ ] T10 Add explicit dirty/keep/set/reset semantics for context, model and roles (F09–F10, F23).

### Keys, profiles and security

- [x] T11 Repair legacy alias handling for direct Apply and generated helpers (F02).
- [x] T12 Share vault resolution across Apply/Test/Fetch/proxy and enforce provider ownership/enabled state (F05–F06).
- [x] T13 Prevent migration from creating phantom providers for indexed/internal secret files (F19).
- [x] T14 Recalculate usable-key availability after add/delete/disable/promote, including all-disabled cases (F08).
- [x] T15 Define and implement what happens to active/referenced keys on disable/delete/promote (F07).
- [x] T16 Await credential saves; preserve failure entries; prevent duplicates; scope async results to provider (F18).
- [x] T17 Update metadata when replacing an indexed secret; test that displayed mask matches the current secret.
- [x] T18 Create durable custom provider profiles with independent IDs, model/endpoint and key groups (F21).
- [x] T19 Add cross-process vault locking and bounded metadata reads; test interrupted writes and orphan cleanup.
- [x] T20 Audit generated scripts, redaction, permissions, error content, sensitive inputs and secret-shaped model IDs.

### Switching engine and protocol

- [x] T21 Persist provider groups and key order; migrate old flat routes without hidden behavioral changes (F15).
- [x] T22 Implement provider-first / key-within-provider candidate selection and per-provider model mapping.
- [x] T23 Separate observed failures from inferred auth/quota health; handle key vs account/provider cooldowns.
- [x] T24 Honor cooldowns when all candidates are unavailable; expose retry time and explicit manual retry (F16).
- [x] T25 Bound attempts, total latency and concurrent proxy work; test concurrent failures and recovery.
- [x] T26 Preserve the no-replay-after-stream-start invariant and document timeout/charging uncertainty.
- [x] T27 Update request/response header and error forwarding against current gateway requirements (F17).
- [x] T28 Validate endpoint composition, including custom `/v1` suffixes; preview final Messages URL.
- [x] T29 Parse loopback hosts consistently, including IPv6 and misleading hostnames (F27).
- [x] T30 Make save/check/enable refer to the same persisted policy, with visible unsaved state (F14).
- [x] T31 Reject unsupported protocols in Rust and UI before any configuration change (F13).
- [x] T32 Keep redacted switch history, cooldowns and current provider/key visible without logging prompts/secrets.

### Desktop usability and accessibility

- [x] T33 Move blocking network/process commands off Tauri's main thread (F03).
- [x] T34 Introduce provider cards with inline keys and explicit provider/key priority controls.
- [x] T35 Add a short setup sequence and review/activate step; simplify the current six-screen dependency chain.
- [x] T36 Consolidate duplicate helper/proxy controls and clarify login-start behavior (F25).
- [x] T37 Allow manual model entry after catalog fetch; preserve intentional selection; show supported discovery actions (F22).
- [x] T38 Replace misleading current/default/verified terminology; show the exact status evidence.
- [x] T39 Implement every section/field `i` help entry in `.sps/help-inventory.md` (F26).
- [ ] T40 Test help hover/focus/click/keyboard dismiss, small-window placement and screen-reader relationships.
- [ ] T41 Review all labels, error descriptions, focus order/rings, contrast, target spacing, zoom and reduced motion.
- [ ] T42 Validate actual native 760×780 and 620×560 layouts, dark/light/system, and slow/error states.

### Tests, dependencies, documentation and DMG

- [x] T43 Make dev/test mocks stateful and schema-faithful; eliminate misleading static success paths (F29).
- [x] T44 Add regressions for F01–F20 through real service/command seams, using isolated fixture homes.
- [x] T45 Exercise grouped switching A1→A2→B1, disabled/deleted keys, all-cooling, retry bounds and SSE aborts.
- [ ] T46 Verify packaged helper survival, login agent, stale PID identity and version upgrades; remove obsolete state safely.
- [x] T47 Audit root npm, Rust target graph and bridge lock independently; distinguish maintenance warnings and exploitable versions.
- [x] T48 Reconcile all current docs and status claims; clearly date historical audits; update the user's guide with the new flow.
- [ ] T49 Complete the remaining file-by-file/manual review and record coverage; audit binary/media assets separately from code.
- [ ] T50 Run frontend, Rust, property/security/crash tests and native acceptance checks after remediation.
- [x] T51 Bump all release metadata together and build a repaired local .app/DMG from a recorded source revision.
- [ ] T52 Verify signature, DMG checksum, mount/copy/launch, installed behavior and rollback; provide SHA256SUMS.
- [x] T53 Record Gatekeeper/notarization truthfully; use Developer ID only if the owner supplies an identity.
- [x] T54 Deliver artifact, release notes, migration instructions and unresolved limitations; publish only when requested.

## Checks actually run

| Check | Result |
| --- | --- |
| Initial git status | Clean; no pre-existing user changes reported |
| npm run check:frontend | PASS: ESLint, Prettier, TypeScript, 201 tests / 27 files |
| npm run check:rust | PASS: fmt, clippy -D warnings, unit and integration suites |
| cargo test -- --list | 337 tests listed; five source-marked live network tests ignored in standard run (332 executed) |
| npm run build | PASS; JS 312.44 kB (94.15 kB gzip), CSS 37.18 kB (7.97 kB gzip) |
| npm audit | Zero reported vulnerabilities at audit time |
| cargo audit | Tool is not installed; no RustSec cargo-audit pass claimed |
| scripts/dep-audit.sh | PASS: 726 resolved packages checked; zero advisories affecting the macOS graph; three Linux/build-only advisory records excluded |
| Separate optional bridge lock audit | Three moderate affected packages: @the-next-ai/ai-gateway, glob, minimatch; npm reports no available automatic fix. No bridge packages installed/executed during this audit |
| Tracked Markdown relative-link scan | Five explicit local Markdown destinations checked; zero broken targets. Plain-text/backtick references, external links and anchors were not validated by this scan |
| Installed vs local release app | Both version/build 1.0.1; executable SHA256 hashes match |
| codesign --verify --deep --strict | PASS on installed app |
| spctl --assess --type execute | REJECTED on installed app |
| hdiutil verify local 1.0.1 DMG | VALID checksum |
| Live GUI/browser/VoiceOver | Not completed: browser host unavailable; native binding timed out |
| Paid inference / full Claude Code conversation | Not run |
| Remote release download / clean Mac / remote CI | Not verified in this audit |

Installed resolutions include React 19.3.1, Tauri API/CLI 2.12.4, Rust Tauri
2.12.5, Vite 8.3.6, Vitest 5.1.0, TypeScript 6.0.3 and ureq 3.5.1. A dependency
upgrade alone is not a demonstrated fix for an empty settings file or these
application-level state/credential defects.

The optional bridge result is separate from the clean root dependency scan.
It is not proof that the currently disabled bridge is exploitable through the
app. Review and repair the pinned tree before enabling that feature, and retain
the integrity/lockfile checks when repinning it.

## Research checked before changes

Only official/primary sources underpin compatibility recommendations. Checked
2026-11-17; these are living pages, not a promise of permanent behavior.

- [Claude Code settings precedence](https://code.claude.com/docs/en/settings):
  the global user file is only one scope; higher-precedence settings can win.
- [Settings reference: apiKeyHelper](https://code.claude.com/docs/en/settings-reference#apikeyhelper):
  helper output is sent in both API-key and bearer headers and is cached.
- [Authentication precedence](https://code.claude.com/docs/en/authentication#authentication-precedence):
  provider selection and environment credentials can take priority over helper output.
- [Model configuration](https://code.claude.com/docs/en/model-config):
  custom context behavior depends on recognized IDs, `[1m]`, and CLI version;
  the app's blanket explanation needs revision.
- [Official Claude Code changelog](https://github.com/anthropics/claude-code/blob/main/CHANGELOG.md):
  current first entry is 2.1.326; context behavior for unknown `claude-` IDs
  changed in the recent version range. Installed version is 2.1.325.
- [Gateway compatibility](https://code.claude.com/docs/en/llm-gateway-protocol):
  inspect supported endpoints, open-ended Anthropic headers, response retry
  headers and unchanged error bodies. Missing token counting is optional;
  do not claim its absence alone breaks inference.
- [OpenRouter integration](https://openrouter.ai/docs/cookbook/coding-agents/claude-code-integration):
  documented bearer-token setup and compatibility caveats; a catalog listing
  is not proof every model works correctly with Claude Code.
- [DeepSeek Anthropic API](https://api-docs.deepseek.com/guides/anthropic_api/)
  and [Z.AI Claude Code setup](https://docs.z.ai/devpack/tool/claude):
  provider endpoint/auth/model guidance checked against embedded presets.
- [Tauri async commands](https://v2.tauri.app/develop/calling-rust/#async-commands):
  ordinary synchronous commands execute on the main thread unless explicitly
  dispatched asynchronously; blocking operations need appropriate dispatch.
- [Tauri macOS signing](https://v2.tauri.app/distribute/sign/macos/) and
  [Tauri CLI 2.12.5 release](https://github.com/tauri-apps/tauri/releases/tag/tauri-cli-v2.12.5):
  distinguish signature validity from notarized distribution. Reviewed release
  notes do not establish a fix for this app's runtime configuration failures.
- [WAI tooltip pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tooltip/)
  and modern-web-guidance `interest-triggered-tooltips`, `accessibility`, `forms`:
  help must be reachable deliberately, dismissible and usable without hover.

## Audit scope and limits

Inventory: 76 frontend files / 9,932 lines; 34 backend source files / 14,540
lines; 10 Rust integration files / 2,517 lines; 114 documentation files /
5,907 lines; one script / 94 lines; one CI workflow / 76 lines. These counts
include tests and the pinned bridge lock where applicable, and exclude binary
assets and installed/generated dependencies.

The audit traced core UI/IPC/credential/configuration/restore/routing/lifecycle
paths, read current and historical documentation, ran the available local
quality gates, examined release artifacts, and inspected local file metadata
without exposing secret values. Not every line in every historical document,
test, dependency or binary has received an independent manual review. T49
keeps that remaining coverage explicit rather than calling it complete.


## Approved remediation evidence — 1.0.2

Full plan batch-approved on 2026-11-17. No repository push or public release.

| Area | Delivered | Evidence / remaining limits |
| --- | --- | --- |
| Current Mac recovery | Empty original archived privately; exact valid backup restored atomically | Object verified; no key or endpoint changed. Origin of empty file and real API connectivity remain unknown. |
| Recovery in app | Explicit valid-backup/clean recovery with confirmation, private invalid archive, shared lock and checked write | Regression tests cover empty original, bad newest backup and valid older backup. Selectable backup preview/details remain incomplete (T03). |
| Credentials | Legacy aliases, ownership/enabled checks, shared resolver, migration filtering, permission/symlink protections, cross-process mutation lock | Fixture service tests pass. Broader metadata-size/orphan/crash review remains (T19–20). |
| Apply | Input/settings preflight; secret/helper/index rollback guard; checked first-run exclusive install; role/context reset intent | Backup failure rollback tested. Not a crash-atomic multi-file transaction; rollback errors in Drop remain best-effort (T05–06). |
| Switching | Provider groups expand enabled keys in priority; explicit legacy targets retain order; models per provider | Real socket A1→A2 (B untouched), A1/A2→B1, all-cooling regressions. Shared account-limit inference remains unsupported (T23). |
| Proxy | Expanded Anthropic headers, preserved caller-error response, bounded attempts/time/concurrency; stream failure reported without replay | Existing protocol/SSE/security suites pass; broader concurrent slow-client stress not certified (T25/45). |
| Desktop commands | Sync blocking work dispatched with Tauri supported `command(async)` | fmt/clippy compile and suite pass; native stalled-network responsiveness still needs observation. |
| Interface | Four screens, combined setup/keys/model, provider cards, awaited add with failure preservation, manual model override, refresh/error states, shared info affordance | 204 frontend tests pass. Native layout/VoiceOver/zoom acceptance is separately recorded; not claimed by DOM tests. |
| Model drafts | Provider switch clears typed key/model/roles/context; custom endpoint hashes scope keys; explicit role/context reset | Saved blank fields deliberately preserve overrides. Cross-provider saved model/role review is still a manual responsibility (T10). |
| Bridge | Unsupported execution disabled in command layer as well as UI; PID identity tightened for dormant lifecycle | Separate optional lock has three moderate affected packages; no bridge install or code execution performed. |
| Documentation | README/PRODUCT/current guide/limitations reconciled; historical pages remain historical | Prior media is not current acceptance evidence; file inventory is not every-line certification (T48–49). |

### Post-repair automated checks

- `npm run check:frontend`: PASS — lint, formatting, TypeScript, 204 tests / 28 files.
- `npm run check:rust`: PASS — fmt, clippy with warnings denied, 345 tests passed,
  five live-network tests ignored, no failed tests (14 result suites).
- Production bundle/signature/DMG checks: see [release evidence](../releases/1.0.2.md).
- Baseline dependency audits above apply to unchanged dependency versions.

### Outstanding acceptance and improvements

Unchecked todos above are retained intentionally. Priorities for follow-up:

1. Native small-window/zoom/VoiceOver, clean-Mac trust/install, actual CLI conversation,
   logout/login and stale-helper upgrade acceptance (T40–42/46/50/52).
2. Select a specific validated recovery snapshot with endpoint/model/helper preview
   rather than newest-valid or clean only (T02–03).
3. Durable provider setup drafts independent of switching policy; explicit saved
   model/role review on provider change (T10/18/35). Current policy activation
   saves before applying, with pending state and save receipts.
4. Stronger multi-file transaction journal and explicit rollback-failure reporting;
   complete metadata read bounds/temp cleanup/orphan stress (T04–06/19/20).
5. Account-aware cooldown only when provider supplies trustworthy scope information,
   HTTP-date Retry-After, durable redacted history, concurrency stress and full
   model matrix (T23/25/32/44–45).
6. Apple Developer ID/notarization and CI owner-side acceptance. No signing secret
   or hosted-release publication is assumed.

This is a substantive repair and broad audit, **not** a claim that every line,
third-party dependency, binary asset and external provider has been certified.
The baseline SHA inventory in `.sps/review-coverage.json` makes the remaining
manual review boundary explicit.


### Additional findings from release verification

- The standard Tauri DMG customization script timed out waiting for a Finder
  AppleEvent (-1712) on this host. The app itself built successfully. Added
  `npm run build:macos`: build the signed app, then package a simple drag-to-
  Applications DMG using the installed Apple disk-image CLI, without Finder
  scripting. The image is verified before replacing the final artifact.
- Predictable `credentials.json.tmp` and `routing.json.tmp` files could be
  followed/truncated. Both metadata writers now reuse the private unique-temp
  atomic writer; readers use bounded regular-file JSON checks. A fixture symlink
  regression proves the unrelated destination stays untouched.
- The shared writer's directory symlink check rejected a nonexistent first-run
  parent. Missing directories are now allowed and created; actual symlinks still
  fail closed. A missing-parent fixture regression covers this.
- Native navigation/help/diagnostics were observed at the default 760×780 window. This caught and corrected
  stale backup-location and first-run copy. Navigation labels now match the
  simplified four-screen workflow. Pending switching edits and save receipts
  are explicit.


### Actual CLI smoke through the packaged helper

`python3 scripts/smoke-cli-loopback.py` used a disposable home/config directory,
`claude --bare`, no tools/MCP/session persistence, fake credentials and a local
HTTP fixture. The signed bundle's helper received Claude Code's request, tried
fixture key 1 (401) then key 2 (200 SSE), and mapped the request to the target's
model. Claude Code 2.1.325 exited 0 and printed `LOOPBACK_OK`; both requests used
`/v1/messages`. This verifies a real CLI/proxy/stream round trip, not a real
provider's model compatibility. Fixture helper/data were cleaned up.

Native Help opened/collapsed in the accessibility tree; the final four-screen
bundle launched and showed corrected copy. A minimum-window resize attempt
failed in computer automation with `windowNotFoundAtPosition`; minimum size,
zoom and VoiceOver acceptance remain unverified. No real keys were edited and
no live Apply was performed during native inspection.

## Save-button follow-up — 1.0.3

Reproduced in the current native window: NVIDIA NIM has stored key and selected
model, but `canApply` rejects its `open_ai_only` protocol. The old footer incorrectly
blamed highlighted fields and the badge pointed to an unavailable bridge setup.
1.0.3 explicitly explains this limitation, removes ready-looking capability
framing for that integration, disables its unsupported key check, and associates
the action with the visible reason. Regression confirms NVIDIA does not write
settings while a supported provider can Apply using fixture IPC. All 205 frontend
tests/lint/format/typecheck pass. Hosted NVIDIA activation remains unimplemented;
no claim that this copy repair implements the missing protocol integration.
The user's current native draft was preserved without restart or real Apply.

## NVIDIA implementation and requested cleanup — 1.0.4

User explicitly requested the missing connection plus uninstall/cache cleanup.
The built-in Rust helper now translates NVIDIA Chat Completions; Save & Apply
supports NVIDIA and retains other groups as fallbacks. Third-party bridge stays
disabled. This supersedes the unavailable-NVIDIA statements in earlier snapshots.
206 frontend tests and 356 Rust tests pass; the real CLI Save/helper/key-failover/
Read-tool/final-answer path passed against a local NVIDIA fixture. Real hosted
inference remains unverified; see the 1.0.4 release limits/evidence.
The installed app, LaunchAgent and disposable cache/WebKit folders were moved
to Trash; old helper stopped. Keys, user settings and backups retained. Source
and local build artifacts remain in the project; no replacement installed or
remote release created. Third-party dependencies unchanged.


## Completion pass — 1.0.5 unsigned prerelease

2026-11-17, under the user's full remaining-scope approval. User explicitly chose
an unsigned prerelease; no signing certificate was supplied. Prior evidence above
is historical. The current release notes/evidence describe the final artifacts.

| Repair | Current evidence |
| --- | --- |
| Selected backup recovery | Bounded metadata-only previews, corrupt entries disabled, exact-name selection revalidated, confirmed restore; backup_selection, backup_path_hardening and RecoverySelector tests |
| Recoverable managed Apply | Private before-image journal; planned new files tracked; explicit restore/keep choice; real child exit without destructors; a second live process excluded; transaction_journal tests |
| Failed activation | Shared policy/key/helper/settings transaction; NVIDIA and grouped activation rollback; identical keys deduplicated; external settings edits retained; repair_regressions tests |
| Private managed paths | Unique exclusive temporaries; bounded no-follow regular-file reads; nested symlink rejection; helper chmod through open descriptors; managed_paths/security tests |
| Helper command correctness | Shell-quoted absolute executable paths; spaces, quotes and substitution characters tested; installed real CLI Read/tool round trip passed from a fixture home containing spaces and quotes |
| Drafts and review | Valid provider model/role/context drafts survive switching/relaunch; endpoint-scoped custom drafts; key input omitted; visible saved/pending review and legacy direct-NVIDIA activation warning |
| Switching observations | 100-event private history excluding keys/labels/endpoints/prompts/responses; HTTP-date and seconds Retry-After; generic account scope stays unknown; bounded concurrency/attempts/phases documented |
| Interface help and visibility | Shared info affordance for every FieldRow/Section; explicit recovery/history/review help; darker control boundaries, visible outline and neutral high-contrast primary buttons |
| Current documents | Guide, product, README, module map and threat model reconciled; obsolete CLIAdapter/Keychain implementation claims removed from current architecture/security pages |
| Security scans | Source text scan and167-commit gitleaks history scan pass; one exact historical fake-loopback-token fingerprint is ignored, with current fixtures locally annotated. npm/OSV macOS graph scan finds0 affected packages. Dormant bridge audited separately. |
| Real account check | OpenRouter authenticated GET /api/v1/key returned200 with system TLS verification. Prior NVIDIA key rejected auth; new minimal inference awaits user readiness. |

T23 does not assert account-wide cooldown detection: no trustworthy account identity
or scope is available from generic errors. Key-level pauses and the explicit
shared-quota warning are intentional. T25 means bounded phases/concurrency/attempts,
not a strict60-second wall-clock ceiling for NVIDIA streaming. T44/T45 represent
fixture/service/socket regressions, not successful hosted inference for every model.

Still not certified: native620×560/200% zoom/VoiceOver, actual logout/login/reboot,
clean-Mac installation/trust, and exhaustive manual per-line correctness. Computer
inspection read the native1.0.5 window and provider screen, but Raise timed out;
the isolated native-copy attempt failed with ScreenCaptureKit -3811. No missing
native test is replaced by an automated PASS. The prior native1.0.4 Save-enabled
observation remains historical. Building/publishing does not reinstall /Applications.

The scope inventory enumerates all git-visible project files, with hashes and
binary/media classification. Targeted source/invariant review plus automated
checks is recorded; it must not be described as proof every line is correct.
T49 remains open for exhaustive manual coverage. T10 retains a narrower gap:
context/roles support explicit resets; blank model keeps the saved model and
removing a model override still requires a deliberate manual edit. Review warns
about retained cross-provider IDs. These are disclosed rather than hidden.


Asset sweep: six app-only historical screenshots and the main icon were visually
inspected; screenshots show obsolete behavior and are not current acceptance
evidence. Two full-desktop T65 captures were removed from the current tree after
inspection because they contained unrelated desktop content; private local copies
were retained and earlier Git history is unchanged. Icon variants and GIF/WebM
assets are inventoried/hashed; full animation frame-by-frame review remains open.


Publication observed: v1.0.5 non-draft unsigned prerelease at GitHub, targeting
source commit de16792964d1929271b074283fb41a49305a34df. DMG, checksum and evidence
assets were downloaded after upload; the remote DMG matches the local final build.
CI run44113582905 passed frontend, Rust and security jobs.


## Final-review patch — 1.0.6

F31: a valid large draft collection could exceed its64KiB read cap, causing all
drafts to be ignored on the next read. Fixed by applying both count/text caps
before writing, evicting oldest entries and retaining recent edits.
F32: a syntactically valid but out-of-range saved history timestamp could throw
in Date.toISOString and break rendering. Fixed with a visible Unknown timestamp
fallback while valid dates keep semantic time markup. Four focused regressions
pass; frontend total219. No backend production-source changes or key operations.
The1.0.5 published artifact is unchanged;1.0.6 is a separate unsigned prerelease.


Final publication: v1.0.6 unsigned prerelease targets
8fc69bdd27543c6fdb6560acf04b317b0b8115da. Downloaded DMG hash matches the local
artifact and SHA256SUMS. CI44115270919 passed frontend, Rust and security jobs.
User-triggered live verification remains blocked before inference pending a
usable stored NVIDIA credential; no successful hosted inference is claimed.
See the publication receipt and source fingerprint evidence.


## Live NVIDIA and active-key replacement follow-up

After user-owned secure entry, one minimal request through the final packaged
1.0.6 helper returned HTTP200 with nonempty text (20 input/3 output tokens).
No live tool/failover coverage is inferred from this basic inference. Supplementary
evidence was uploaded and release notes updated; the binary/checksum is unchanged.
Native inspection confirmed the replacement key saved first, and explained the
old-entry delete refusal: saved direct settings still reference it. The tested
NVIDIA model was entered as a draft; Save & Apply was observed ENABLED. User
activation and old-entry deletion remain pending. Guide now explains this sequence
and that a second API key/account is unnecessary. No real credential deleted.


## 2026-11-18 follow-up: NVIDIA conversation role and helper ownership

Observed: the activated old helper returned its local `unsupported message role`
guard. Current Claude Messages supports inline system instructions, whereas the
NVIDIA endpoint expects system text first. 1.0.7 hoists active text instructions,
retains order and `clear_at` expiry, and rejects unsupported tool-state/effort
changes explicitly. Six adapter and one socket regression pass. Minimal hosted
synthetic system-message inference returned200; this is not the user's original
conversation acceptance. The user key is valid and was not replaced by the agent.

Two old owned helpers responded on different ports; the configured port differed
from the state file. 1.0.7 holds a private OS file lease for the entire helper
lifetime; duplicate raw startup exits before bind/state changes. A focused process
regression and existing crash/restart tests pass. Full Rust409, six ignored default,
frontend219. Final release evidence records package and active-helper checks.

Still open: user retry of the actual conversation, accessibility/minimum native
window/zoom, reboot/login/clean Mac, explicit model-override removal UI, exhaustive
manual per-line/animation review. No blanket completion claim follows from counts.
