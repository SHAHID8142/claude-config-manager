# T32 — Credential handoff via apiKeyHelper + Keychain (STOP-gated task: RESOLVED, no STOP needed)

**Date:** 2026-11-08 · **Result:** ✅ PASS · **Artifacts:** `src-tauri/src/credential_handoff.rs`, config_merge integration

## The verified design (why no STOP was required)

The STOP condition applies only if secure persistent handoff is **unresolved**. It is resolved by an officially documented mechanism, verified twice (T02 matrix + fresh 2026-11-08 doc fetches):

1. **`apiKeyHelper`** — documented as "a shell script that returns an API key"; key printed on stdout (empty output/timeout/error ⇒ failure within three attempts); >10 s triggers a UI warning (our script is stateless `security find-generic-password … -w`, ~10 ms); works in the CLI **and the VS Code extension** (the PRD's primary user context).
2. **Precedence fact with design consequence:** helper credentials rank **below** `ANTHROPIC_AUTH_TOKEN`/`ANTHROPIC_API_KEY` env vars ⇒ the two modes are **mutually exclusive**, enforced in the merge layer; setting both would silently shadow the helper.
3. **Flow:** credential stored in macOS Keychain under `com.claude-config-manager.<provider>` (T22 verified) → app generates a **fixed-template helper script** embedding only the validated provider id → script installed 0700 in the app-owned helpers dir → settings.json receives `"apiKeyHelper": "<absolute path>"`. **The secret never exists in settings.json** (F08 satisfied for the helper path).
4. **Remaining plaintext case:** providers whose endpoints are Bearer-only (e.g. OpenRouter) may need `ANTHROPIC_AUTH_TOKEN` env — plaintext with disclosed tradeoff, gated behind explicit UI consent (lands with Save & Apply, T33). Helper mode serves x-api-key-accepting endpoints (Anthropic native; DeepSeek verified accepting both header styles).

## Implementation
- `validate_provider_id` ([a-z0-9-]{1,64}) gates every entry point — **no shell injection surface** (threat T6; adversarial ids tested: spaces, metachars, `$(…)`, backticks, traversal, oversized).
- `generate_helper_script`: fixed template, `exec security find-generic-password -s "com.claude-config-manager.<id>" -a "<id>" -w`; takes **no secret input** (asserted).
- `install_helper_script`: 0700 dir + 0700 script, atomic temp+rename, permissions verified post-install, idempotent replace, no temp residue (asserted).
- `uninstall_helper_script`: idempotent.
- **Merge integration:** `apiKeyHelper` added to `MANAGED_TOP_KEYS`; `DesiredSettings.api_key_helper` with validation (absolute path required — env values are not shell-expanded per T02; provider-script stem check as defense in depth); mutual-exclusion rule vs `env_api_key`/`env_auth_token` with tests.

## Security review
- Secret never in script/settings/logs (generation input-proofed); injection-proofed via id validation on all paths; permissions enforced and verified; helper lifecycle (install/replace/uninstall) tested.

## Tests (evidence)
- 5 handoff tests (fixed shape, no-secret scan, 8 adversarial ids, 0700 install + replace + uninstall, residue check) + 4 merge tests (helper write, **mutual exclusion ×2**, relative-path rejection, invalid-stem rejection).
- `cargo test` **111/111**; clippy -D warnings clean (1 collapsible-if fix, cycle 1); fmt clean.

## Acceptance criteria
- [x] Handoff implemented per verified docs; secure persistent handoff RESOLVED (no STOP)
- [x] Bearer-only plaintext fallback documented as consent-gated follow-up
- [x] Negative + adversarial tests
