# Repair and usability plan

Status: approved — full batch approval received 2026-11-17.

## Outcome

A user can add providers and multiple keys, reorder providers and their keys,
choose a model for each provider, enable automatic switching once, and see which
provider/key is serving requests and why a switch occurred. Invalid settings
lead to a recovery action rather than a dead end. Deliver a newly versioned,
locally verified DMG with accurate installation and limitation documentation.

Success example: Provider A key 1 → A key 2 → A key 3 → Provider B key 1 → B key 2.
A healthy first key serves normally. A failed key is skipped according to a
documented bounded policy. Exhaustion has a readable explanation and retry time.
The user never has to create one routing target per key.

## Known / assumed / unverified

- Known stack and scope: profile.md. No CMS, server deployment, SEO, or public
  website is involved. Do not apply marketing-site section/hero rules here.
- Known data: private files, metadata index, legacy aliases, routing v1,
  settings backups, detached helper and optional LaunchAgent.
- Assumed reasonable default: preserve strict provider priority and key
  priority; skip unavailable keys; recover higher-priority keys when their
  cooldown expires. Do not permanently mark a key dead after a temporary fault.
- Unverified: live service compatibility, clean-Mac Gatekeeper flow,
  VoiceOver, installed WebView layout, logout/login survival, remote CI status.
- No paid provider calls are needed for the fixture-based repair tests.
- Single agent. No repository push, GitHub release, or remote publication.

## 1. Recovery and truthful status

Files: config_reader/status/restore/save_apply, IPC types, App, Dashboard,
Maintenance, regression suites.

- Preserve the empty file before recovery. Offer preview of the newest valid
  backup, including endpoint/model/helper and backup timestamp. Do not silently
  treat every malformed file as `{}`. For the current machine the candidate is
  `settings-backup-1794890788644002000.json`; endpoint validity remains separate.
- Add an explicit recovery action: restore a selected valid backup, or create
  a clean settings object after preserving the invalid bytes. Repair uses the
  same locking, symlink checks, private permissions, and conflict protection.
- Show read error with Retry/Recover; load/error/success are distinct states.
- Refresh status, environment, backup metadata and helper state after writes,
  restores, window refocus and explicit refresh. Do not say “live” without this.
- Show Apply outcomes/errors in the active screen, not only on Dashboard.
- Verification: empty/malformed/missing/permission/shape/symlink recovery;
  latest corrupt backup; valid older backup; concurrent edit; restore refresh;
  original invalid bytes preserved; no secret displayed.

## 2. Credential and transaction correctness

Files: credential_vault/index/migration/service/handoff, save_apply, lib command
handlers, model_discovery_service, connection_test, App and credential forms.

- Centralize provider/credential resolution for Apply, tests, discovery and
  proxy; verify provider pairing, enabled state, file existence and permissions.
- Resolve `<provider>-legacy` metadata aliases to the real provider secret for
  both existence checks and helper generation. Do not synthesize providers from
  indexed credential filenames or internal proxy-token files.
- Validate current settings and all input before mutating helper/secret files.
  Stage side effects; restore previous secret/helper state on failed commit.
- Make enable/promote/delete behavior explicit for active credentials and
  refresh availability after every mutation. Protect referenced active keys.
- Await Add; show pending state; preserve form on failure; clear only on
  success. Update masked metadata when replacing an indexed key.
- Move bounded blocking work off Tauri's UI thread with supported command
  dispatch/spawn_blocking; do not run blocking network work directly on async
  runtime executor threads.
- Verification: legacy-only, indexed-only, disabled/deleted keys, wrong-provider
  IDs, failed backup/write/verification, form rejection, switched-provider races,
  slow network with responsive GUI.

## 3. Provider-first automatic switching

Files: routing_config, failover, proxy, helper, IPC schema and tests.

- Introduce ordered provider groups referencing enabled keys; model is
  configured per provider. Keep endpoint/protocol validation authoritative in
  Rust. Preserve and migrate existing flat chains without silently reordering
  a deliberately interleaved old chain; preview any necessary regrouping.
- Within a provider, use key priority. Continue to the next provider only when
  the current group has no usable key. Distinguish account-wide and key-level
  faults where the response supplies enough evidence; otherwise say unknown.
- Classify invalid/revoked, no credit, temporary rate limit, provider/network
  failure, incompatible model, and invalid caller request separately. A model
  configuration error should not be presented as an exhausted API key.
- Respect Retry-After, cap total attempts and total elapsed time. Do not
  re-attempt every cooling key merely because all keys are cooling. Return a
  useful exhausted/retry-later result with an explicit manual retry control.
- Never replay after response streaming begins. A transport timeout can leave
  upstream processing uncertain; do not promise that retries prevent charges.
- Pass supported Anthropic request headers/body and required response retry,
  rate-limit and stream headers according to current official gateway docs.
  Strip hop-by-hop headers and replace upstream credentials deliberately.
- Record redacted last switch reason, provider/key label, attempt counts,
  cooldown deadline and timestamps. No request bodies or key values in history.
- Keep detached-helper survival; make auto-start opt-in. Ensure settings never
  point at a stopped/misidentified process after lifecycle operations.
- Verification: A1 fails → A2 succeeds (B untouched); all A keys fail → B1;
  disabled keys skipped; all cooling; Retry-After; shared account limits;
  mixed model mappings; 400/422 stop; 401/402/403/429/5xx policy; truncated SSE;
  concurrent requests; app closure; helper restart; schema rollback.

## 4. Simple provider workflow and information buttons

Files: App, all tabs, shared FieldRow/Section/help components, forms, styles,
mock IPC and frontend tests. React performance skill applies when implementing.

- Primary screens: Overview; Providers & keys; Automatic switching; Recovery &
  diagnostics. Advanced model role/context controls stay collapsed in provider
  editing. Reuse existing components where practical.
- Each provider card shows ordered masked keys, chosen model, enabled state,
  test state and actions. Add key is inside its provider. Show both key and
  provider order in a plain preview: A1 → A2 → B1.
- Use clear distinctions: Saved settings / Pending changes / Last tested.
  Review changes before activation; one primary “Apply configuration” or
  “Enable automatic switching” action appropriate to the screen.
- Preserve dirty fields intentionally. Provider switch must not carry another
  provider's model/roles/key. Blank fields keep saved values; resets are explicit.
- Keep manual model entry even after fetching a catalog. Hide unsupported
  discovery/testing actions with a clear explanation. Disable unsupported
  OpenAI-only setup before Apply; do not ship an unusable bridge as ready.
- Implement the section/field inventory in help-inventory.md using one
  accessible `i` affordance: hover, focus, click/tap, Enter/Space, Escape,
  outside dismiss, readable placement and small-window collision handling.
  Keep validation errors and recovery-critical information inline.
- Remove duplicate runtime/proxy control blocks and storage implementation prose
  from primary paths. Keep details available under diagnostics/help.
- Verification: actual Tauri WebView at 760×780 and 620×560, larger display,
  200% zoom, keyboard/VoiceOver, light/dark/system, reduced motion, delayed and
  rejected IPC, no clipped help, no nested interactive labels.

## 5. Documentation and release gates

- Complete the todo register in docs/audits/2026-11-17-project-review.md.
- Reconcile README, PRODUCT, USER_GUIDE, compatibility matrix, architecture,
  security model and V2/V3 limitation/status pages. Historical audits remain
  historical; add dates and avoid using them as current acceptance evidence.
- Correct false byte-preservation/full-transaction/no-double-charge claims and
  backup locations. Document real automatic switching and migration behavior.
- Audit root npm, Rust macOS graph, and bridge lock separately; distinguish
  build-only risks, unmaintained warnings and exploitable shipped advisories.
- Create 1.0.2 (or next chosen version) consistently across manifests and lock
  metadata. Rebuild .app and DMG from the repaired source, record source revision
  and dirty-state status, generate SHA256SUMS and release notes.
- Required checks: frontend lint/format/typecheck/test/build; Rust fmt/clippy/
  unit/integration/property/crash/security tests; dependency audit; app bundle
  codesign verify; DMG verify; mount/copy/launch; installed native workflow;
  helper survives GUI exit; stale-helper upgrade; rollback/recovery smoke test.
- Developer ID/notarization requires the owner's Apple signing identity. Do not
  claim Gatekeeper acceptance with an ad-hoc signature. Deliver accurate local
  installation guidance if that identity is unavailable; do not bypass system
  security checks automatically.
- No publication or overwrite of the current installed app without the user's
  relevant instruction. Deliver the new local DMG and evidence first.

## Approval gate and delivery

SPS SKILL.md explicitly requires: “Plan-First Zero-Code Gate ... stop — zero code
before user approval.” The first-contact audit also says to show the report
before building anything. This is a skill requirement, not a new safety rule.
Approve this plan to begin section 1. `Batch approve this plan` permits consecutive
approved repair sections with checks and progress updates under SPS batch mode.


## Follow-up: unsupported-provider Save explanation

Within the existing batch approval for truthful unsupported-provider UI.
Known: selected NVIDIA NIM has stored key/model; `canApply` rejects OpenAI-only
presets. Replace the misleading field-error/footer and bridge-setup claim with
a persistent explanation; hide ready/verified framing for unavailable integration;
disable its unsupported key test. Preserve user draft/keys and backend guard.
Verify unsupported-provider no-write path and supported-provider Save with mock
IPC, lint/typecheck/build. Package a new local 1.0.3 candidate; preserve 1.0.2.
No real paid tests, current-window restart, installed-app replacement or push.

## Approved NVIDIA implementation and uninstall — 2026-11-17

User explicitly requested “add it ASAP” plus uninstall installed app/clean caches.
Implement an in-process NVIDIA Chat Completions adapter, not the dormant external
package: Anthropic request text/images/tools/results -> NVIDIA, streaming text
and parallel tool arguments -> Anthropic SSE, JSON response support, usage/finish
mapping, bounded parsing, no midstream replay. Save creates/updates the NVIDIA
provider group and activates local routing (helper needed for translation).
Existing provider/key failover remains authoritative. Unsupported content or
features fail clearly rather than being silently dropped. Preserve caller model
and role controls via routed model mapping; credentials stay private local files.
Verify with real socket fixtures and actual CLI/tool round trip to fake NVIDIA,
then all quality checks and rebuilt 1.0.4 app/DMG. No real paid call without new
explicit authorization. Current docs checked before implementation.
Uninstall /Applications app and shared disposable cache/WebKit/saved-state dirs,
remove its login agent and stop its identified helper; move items to Trash and
preserve credentials/settings/backups. Current settings do not point at loopback,
so stopping this old helper does not require rewriting CLI settings. No Git push.

## Finish remaining audit and release — explicitly requested 2026-11-17

User instructed “finish all Still outstanding”, including real provider checks,
remaining recovery/transaction/usability work, native checks and commit/push/release.
Existing full batch approval applies. Publication authorization is now present;
user approved an unsigned prerelease because no Developer ID identity exists
locally. Live NVIDIA retry waits for the user to confirm their key update. Never mark missing external acceptance as passed.

Current approved section: recovery reliability (backup selection + managed-file
transaction recovery). Per SPS parallel-lane playbook: root owns transaction
backend, App/lib integration and .sps; backend lane owns config_restore and its
regressions; UI lane owns new RecoverySelector and its tests only; verification
lane reviews pending source read-only. No CMS. Lanes converge before next section.
This updates the prior single-agent execution assumption under the applicable
SPS playbook; scope, authorization and shared-file ownership remain explicit.

Next sections sequentially: durable drafts/activation review/history + residual
hardening; full manual/source/doc/security/native acceptance; final build/version,
secret scan, commit/push and appropriately classified GitHub release. Do not
restart the user's Mac, weaken security or invent a signing identity. Real
provider validation is in the user's requested remaining scope; use minimal
bounded requests with already stored keys and no private prompt content. Missing
usable credentials/certificates/device acceptance must be reported concretely.


## Follow-up1.0.6 hardening — existing batch scope

Preserve valid drafts at the storage cap by evicting oldest entries before writing
(32 entries/64KiB text cap, most recently edited retained); handle invalid history
timestamps without throwing during render. Add focused regressions. Version and
rebuild1.0.6 as a new unsigned prerelease; keep published1.0.5 immutable. Root owns
these small shared UI/storage edits. No new dependency, key action or paid request.


## Message-role400 repair — approved NVIDIA follow-up

User reports real CLI400 unsupported message role after successful activation.
Known: adapter accepts only user/assistant inside messages; current official
Anthropic docs support mid-conversation system messages. NVIDIA expects an initial
system instruction. Reproduce with actual saved NVIDIA model and both bare/default
CLI fixture behavior; do not log real request bodies. Translate supported system
text deliberately, preserve instruction order, reject unsupported scoped/tool-state
semantics clearly rather than silently dropping. Add socket/real-CLI regressions,
verify a minimal hosted case if necessary under existing live-check authorization.
Build/publish next unsigned patch, preserve previous artifacts, verify and restart
only identified owned helper after build so user retries against repaired code.
Root owns this focused backend/fixture section; no UI or credential changes.


Runtime observation2026-11-18: two identified app helper processes, configured
port53054 differs from recorded helper port61446. First reload refused before
mutation. Add lifetime OS file lease to run_helper (private/no-follow regular file,
nonblocking File::try_lock; release on process death), not timestamp stale locks.
Verify duplicate raw helper startup cannot overwrite state or bind another port,
and existing crash/restart tests still pass. Rebuild7 before consolidating only
attested old helper PIDs on the configured port; settings/secrets must not change.


## Ordered model fallback — requested 2026-11-18

User explicitly requested multiple model selection and automatic next-model fallback.
Implement under existing full batch approval and this concrete feature request. Root
owns this section; no new delegation. Keep current keys and settings untouched.

- Each provider has a primary model and up to4 ordered fallback models. Catalog
  suggestions plus manual IDs, Add/Remove/Move buttons, preview and info help.
- Preserve old single-model configs with optional `fallbackModels`; validate IDs,
  duplicates and limits in Rust. UI saves explicitly, never on catalog selection.
- Provider -> model -> ordered enabled keys. Auth/credit failures pause that key
  across its models; documented model-unavailable errors pause that model across
  keys; transient failures try the next combination. Generic malformed400/422
  and unsupported content stop; never replay after streaming starts.
- Keep total attempt/time bounds. Independent key/model cooldown identities prevent
  a failed model disabling healthy alternatives. Status exposes last selected model.
- Preserve fallback choices when provider setup updates its primary model. A new
  helper version must be started before saving a policy older helpers cannot parse.
- Verify legacy round-trip, bounded/duplicate validation, provider/model/key order,
  model failure vs bad request, cooldowns across requests, auth skip, no replay,
  UI catalog/manual/add/reorder/remove and save persistence with fixture homes.
- Build next unsigned candidate1.0.8; publish under existing release authorization.
  Do not waive open picomatch development-tool advisory or imply all CI passed.

Research before implementation: NVIDIA Chat Completions requires one `model` per
request (https://docs.api.nvidia.com/nim/reference/nvidia-nemotron-3-ultra-550b-a55b-infer);
ordered fallback is orchestrated locally. Anthropic distinguishes malformed requests
from provider availability (https://platform.claude.com/docs/en/api/errors).
OpenRouter describes ordered model fallbacks and returned serving model
(https://openrouter.ai/docs/guides/routing/model-fallbacks); this app also supports
NVIDIA/local routing rather than passing OpenRouter-only fields to other providers.
Modern-web-guidance forms guide read; use native controls, labels, identifier input
hints, explicit saves and inline feedback. React guideline: derive list state, no
new library, catalog load on explicit action and cancel stale provider results.


## Install and final release classification — requested2026-11-18

User explicitly instructed push, install on this Mac and release the final version,
then continue. Existing1.0.8 binary/source tests and unsigned distribution approval
remain applicable. Install the checksum-verified release into /Applications using
a staged verified copy. Preserve settings, key files, policy and any existing app
by reversible handling. Inspect current GUI for pending edits before closing;
launch and verify the installed8 window and owned helper. No new API keys or paid
requests. Do not weaken macOS protections or bypass a trust prompt.

Promote the existing verified1.0.8 GitHub asset/tag to a normal latest release under
the user's explicit final-release instruction. This metadata classification does
not certify all security/native checks. Keep unsigned/no-notarization and open
picomatch development-tool security-CI failure prominent; do not waive the gate,
claim all-pass, change the original binary/tag or fabricate signing. Publish a
finalization receipt distinguishing release classification from acceptance limits.
User's explicit publication instruction takes precedence over maintaining the
previous prerelease classification; no repeated routine approval required.
