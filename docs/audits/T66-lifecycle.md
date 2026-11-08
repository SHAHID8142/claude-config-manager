# T66 — Helper restart, GUI closure, clean uninstall and backup restore

**Date:** 2026-11-08 · **Result:** ✅ PASS (automated) for helper/closure/restore; ⚠️ uninstall is
**manual-only** (no automated command yet — T58 blocked) · **Artifact:** `src-tauri/tests/lifecycle.rs`

## What was verified — by executing the real artifact, not by description
The tests install the **generated helper script** itself and run it against a fake `security` binary on
PATH. That exercises the actual file Claude Code will invoke.

| Concern | Test | Result |
|---|---|---|
| Helper "restart" (repeated invocation) | run twice, compare output | ✅ identical output, no carried state |
| Helper prints only the secret | `found` fake | ✅ stdout is exactly the key; **stderr empty** |
| Helper fails closed when the item is gone | `missing` fake (exit 44) | ✅ non-zero exit, **empty stdout** (no blank credential) |
| Diagnostics never contaminate the secret stream | `noisy` fake (writes stderr) | ✅ secret on stdout, diagnostics on stderr; `exec` keeps streams separate |
| Script file hygiene | mode check | ✅ `0700`, contains no secret material |
| GUI closure cannot abort a write | static guard on `save_apply.rs` | ✅ no `thread::spawn`/`tokio::spawn`/`spawn_blocking`/subprocess — the apply pipeline is synchronous, so closing the window cannot interrupt it |
| Backup restore after an apply | create backup → modify → restore | ✅ previous configuration restored |
| Uninstall keeps user data | manual key removal on a realistic config | ✅ managed keys gone, `permissions.allow` and other user keys preserved |

## Clean uninstall — what exists today (honest)
**There is no uninstall button or command.** Automated uninstall belongs to the bridge work (T58), which is
blocked on user consent for the bridge binary. Until then the documented manual procedure is:

1. In `~/.claude/settings.json` (or `$CLAUDE_CONFIG_DIR`), delete `model`, `apiKeyHelper`, and inside `env`
   the `ANTHROPIC_*` keys this app manages. Everything else is user data and must stay.
2. Delete `~/Library/Application Support/ClaudeConfigManager/helpers/` (the generated scripts).
3. Optionally delete `~/Library/Application Support/ClaudeConfigManager/backups/` (backups are the only
   recovery path — deleting them loses rollback).
4. Remove the Keychain item: `security delete-generic-password -s com.claude-config-manager.<provider> -a <provider>`.
5. Remove the app bundle.

The test `uninstall_leaves_no_managed_keys_but_preserves_user_data` encodes step 1 as the contract, so the
guide cannot drift into telling users to delete settings the app never wrote.

## Security review
- The helper passes no user-controlled string to a shell: it `exec`s `security` with the validated provider
  id as a quoted argument (T59's id rule — no leading `-`, `[a-z0-9-]` only).
- Failure path emits **nothing** on stdout, so a broken Keychain item cannot be mistaken for an empty key
  and cannot silently fall back to "no auth".
- Uninstall cannot be triggered by the app automatically, so there is no destructive path a compromised
  renderer could reach.

## Tests (evidence)
- `cargo test --test lifecycle` → **6 passed**
- Full suite: **156 unit (+2 ignored) + 7 adversarial + 7 crash + 6 lifecycle + 14 malformed + 11
  secret/network = 201** · `cargo clippy --all-targets -- -D warnings` ✓ · `cargo fmt --check` ✓

## Acceptance criteria
- [x] Helper restart/idempotency verified against the real script
- [x] GUI closure proven irrelevant to write integrity
- [x] Backup restore verified end-to-end
- [x] Clean uninstall: documented procedure + contract test; **no automated command** (disclosed)
- [ ] Real macOS uninstall/launch on a clean machine — **NOT TESTED** (T65)

## Known limitations
- The uninstall procedure is manual and involves editing JSON; a malicious/typo edit could remove user keys
  (mitigated by the backup taken on every apply — restore is one click).
- Keychain item cleanup is per provider and not enumerated by the app; users must remember each id.
- Helper execution was verified with a fake `security`; the real `/usr/bin/security` interaction is covered
  only by the T22 real-Keychain test, not by this suite.
