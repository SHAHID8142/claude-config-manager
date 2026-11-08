# G11 + G12 — the two defects found by actually running the app

**Date:** 2026-11-08 · **Reported by the owner:** (1) *"after save it's gone black screen and it
cannot load the model capabilities"*; (2) *"Save & Apply failed. credential vault error: keychain access
refused by the OS"* · **Both reproduced, both fixed, both verified in the running bundle.**

---

## G12 — black screen after Save (severity: critical, all users)

### Reproduction
Captured the real wire payloads from the Rust DTOs and rendered `<App>` against them
(`src/wireContract.test.tsx`). It threw immediately:

```
TypeError: Cannot use 'in' operator to search for 'helperShadowedByEnv' in helperShadowedByEnv
    at warningText (src/App.tsx:59)
```

### Root cause
Serde serialises an enum **unit variant as a bare string**:

```json
"warnings": ["helperShadowedByEnv", {"plaintextKeyInEnv": {"which": "ANTHROPIC_API_KEY"}}, …]
```

The UI typed every warning as an object and used `"helperShadowedByEnv" in w`. The `in` operator on a
primitive throws a `TypeError` **during render**; React tears down the whole tree, so the window goes
blank/black with no message. Any settings file containing a shadowed helper — i.e. exactly the state a
first Save creates — hit it. The UI was never rendered against a real payload before, so nothing caught it.

### Why the existing tests passed
Every fixture used `{ helperShadowedByEnv: null }` — a shape the backend never produces. The tests agreed
with the test double, not with the wire.

### Fix
- `StatusWarning` now models the real union: `"helperShadowedByEnv" | { plaintextKeyInEnv } | { modelConflict }`.
- `warningText` guards with `typeof w === "string"` before any `in` check.
- **Second, related defect fixed in the same place:** `ModelConflict`'s fields were not camel-cased
  (`#[serde(rename_all)]` on an enum renames variants only), so the warning rendered literal
  `settings say "undefined"`. Added the attribute on the variant plus a Rust contract test asserting the
  exact JSON the UI reads.
- **Black screens are now impossible-by-construction:** `ErrorBoundary` wraps the app and shows the error,
  a reload button, and the reassurance that the settings file was not changed.

### Verification
- `src/wireContract.test.tsx` (2 tests) renders the app against **payloads captured from the Rust DTOs**,
  including the save-then-refresh path, and asserts the words `undefined` never appear.
- `src/components/ErrorBoundary.test.tsx` (2 tests).
- **Seen in the running bundle** (screenshot inspected, then deleted because it contained the owner's
  config): warnings read
  `Model conflict: settings say "nvidia/nemotron-3-ultra-650b-a55b" but ANTHROPIC_MODEL says "auto" (env wins).`
  — real values, no `undefined`, no blank window.

---

## G11 — "keychain access refused by the OS" on Save & Apply

### What was established
| Experiment | Result |
|---|---|
| `KeychainStore` from an unbundled terminal binary | ✅ set/get works |
| Same code copied into the app bundle and run | ✅ works |
| ad-hoc signed, and ad-hoc + hardened runtime | ✅ both work |
| Updating an item created by a foreign binary (`security` CLI) | ✅ works |
| Keychain items present for our services | `com.claude-config-manager.nvidia-nim` **exists**; others absent |

So the refusal is environment-specific and could not be reproduced here. **The original error message made
diagnosis impossible**: our mapping replaced both `PlatformFailure` and `NoStorageAccess` with the fixed
string *"keychain access refused by the OS"*, discarding the OSStatus.

### Fix (two parts)
1. **Diagnosability:** the platform detail (an OSStatus such as `-25293`) is now included, passed through
   `strip_secret_shaped` first. `platform_refusals_keep_the_os_cause_but_drop_secrets` proves the cause
   survives while a credential-shaped token does not.
2. **Degradation instead of failure:** new `FallbackStore` tries the Security framework first (the secret
   never leaves the process) and, **only on a platform refusal**, retries with `/usr/bin/security` — the
   same mechanism the generated `apiKeyHelper` uses, so write and read paths finally agree. A missing item
   or invalid identifier does **not** trigger the fallback.

`SecurityCliStore` sends the secret on **stdin** (`security add-generic-password -w` prompts twice), never
on argv, so it cannot be read from `ps`; `/usr/bin/security` is addressed by absolute path so a hostile PATH
cannot answer for it; service/account names are validated (non-empty, ≤255, no leading `-`, no control
characters) before anything is spawned.

### Verification
- `secret_is_passed_on_stdin_not_on_the_command_line` — a recording fake proves argv contains only
  `add-generic-password -U -a … -s …` and the secret arrives on stdin.
- `missing_item_maps_to_none_not_an_error`, `failures_surface_the_platform_message_without_the_secret`,
  `hostile_identifiers_are_refused_before_spawning_anything` (nothing spawned).
- `a_platform_refusal_falls_back_and_the_save_succeeds`, `a_missing_item_does_not_trigger_the_fallback`,
  `both_failing_reports_the_first_cause_too`.
- **Real keychain** (`cargo test --test keychain_fallback -- --ignored`, run and passing): with the
  framework path forced to refuse, the fallback writes, reads back the exact value, and deletes through the
  real login keychain.

### Still unknown (stated, not hidden)
The *underlying* reason macOS refused `SecItemAdd` for that build is not identified. The app now works
anyway and reports the OSStatus if both paths fail. If you hit it again, the message will name the code.

---

## Gates after both fixes
```
cargo test        236 passed, 0 failed, 4 ignored (live network / real keychain)
cargo clippy -D warnings, cargo fmt --check    clean
npx vitest run    124 passed / 14 files    lint, format, tsc, build   clean
```
