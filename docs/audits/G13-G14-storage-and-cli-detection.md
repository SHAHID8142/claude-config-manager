# G13 — credential storage: Keychain removed, owner-only file installed
# G14 — "Claude Code CLI not found on PATH" false alarm

**Date:** 2026-11-08 · **Trigger:** owner report — *"we cannot use the Keychain feature… we need to switch
to a better, simpler secured method"* — plus the app showing **"Claude Code CLI not found on PATH"** while
`claude` was installed at `~/.local/bin/claude`.

---

## G13 — storage switch

### Why the Keychain had to go
`SecItemAdd` was refused for the shipped bundle (`-25293`, an entitlement/ACL class of failure). Reproduced
attempts that **did not** explain or fix it: unbundled binary, bundled binary, ad-hoc re-sign, ad-hoc +
hardened runtime, a foreign-owned item, `security` CLI writes. Since the feature is the app's core, the
storage mechanism was replaced rather than cosigned around.

### What replaced it
`src/file_secret_store.rs` — credentials at

```
~/Library/Application Support/ClaudeConfigManager/secrets/<provider>.secret
```

| Property | Implementation |
|---|---|
| No group/world access | directory **0700**, file **0600**, both set at creation (`OpenOptionsExt::mode`), never chmod-after-write; `create_dir_all` result re-tightened |
| No partial files | temp + `fsync` + `rename` in the same directory |
| No symlink attacks | `symlink_metadata` refuses a symlink in **both** directions (read and write); the directory itself is checked too |
| No silent loosening | a file whose mode has group/other bits is **refused on read**, and the helper script refuses it too (`stat -f %Lp` = `600`) |
| No name escape | the filename comes from the **account** validated as `[a-z0-9-]{1,64}` (no leading `-`), and the service must equal `com.claude-config-manager.<account>` — so two ids can never share a file and no path separator can appear |
| No empty/garbage secrets | empty, control-bearing, or >16 KiB values are refused |
| Outside `~/.claude` | a dotfiles sync of `~/.claude` cannot pick the credential up |

**Helper script** now reads the file and fails closed:

```sh
f="$HOME/Library/Application Support/ClaudeConfigManager/secrets/<id>.secret"
[ -f "$f" ] || { echo "credential file missing for <id>" >&2; exit 1; }
[ "$(stat -f %Lp "$f")" = "600" ] || { echo "credential file permissions are too open: $f" >&2; exit 1; }
exec cat "$f"
```

`<id>` is the validated provider id; `$HOME` is expanded at request time, so no absolute path is baked in.

### Removed
`keychain_store.rs`, `security_cli_store.rs`, `fallback_store.rs`, `tests/keychain_fallback.rs`, and the
`keyring` dependency. `scripts/dep-audit.sh` still passes with a smaller graph.

### Honest security statement (also in the user guide)
This protects against other users, cloud sync of `~/.claude`, logs and accidental world-readable states.
It does **not** protect against another process running as the same user — identical to `~/.ssh/id_ed25519`,
`~/.aws/credentials` and `~/.config/gcloud`. macOS offers no API to encrypt a secret against the same user;
a key stored beside the ciphertext would be theatre, so the file is not encrypted and the docs do not claim
it is.

### Tests (all passing)
`round_trips_a_secret_and_creates_owner_only_files` (asserts 0600/0700) · `replacing_keeps_exactly_one_file_and_no_temp_residue` ·
`a_loosened_file_is_refused_rather_than_trusted` · `a_symlink_is_never_followed_in_either_direction` ·
`hostile_identifiers_cannot_escape_the_directory` (7 rejections + the legitimate pair resolving to
`deepseek.secret`) · `empty_or_control_bearing_secrets_are_refused` · `delete_is_idempotent`, plus the
rewritten lifecycle tests that execute the **real generated script**: prints only the credential, restart
idempotent, fails closed when the file is gone, refuses a 0644 file with nothing on stdout.

---

## G14 — CLI detection false negative

`find_claude_cli()` searched **only `PATH`**. A GUI app launched from Finder/Dock inherits
`/usr/bin:/bin:/usr/sbin:/sbin` — not the shell's PATH — so an install at `~/.local/bin/claude` was
invisible and the UI claimed it was missing. (It looked fine in earlier terminal-launched tests.)

### Fix
1. PATH first (so an explicit PATH choice still wins), then real install locations:
   `~/.local/bin`, `~/.claude/local`, `~/.npm-global/bin`, `~/.bun/bin`, `~/.volta/bin`,
   nvm/fnm version dirs (newest first), `/opt/homebrew/bin`, `/usr/local/bin`, `/usr/bin`.
2. Candidates must be **regular files with an execute bit** (a 0600 file named `claude` is not a CLI).
3. The `--version` probe runs with a **repaired PATH** (Homebrew, `/usr/local/bin`, the binary's own dir):
   the CLI is `#!/usr/bin/env node`, so probing with the GUI PATH failed with ENOENT even after discovery.
4. Last resort: ask the login shell (`zsh -lc`, then `bash -lc`) for `command -v claude`, accepting only a
   path inside the user's home.

Tests: `cli_is_found_in_the_usual_install_locations_without_a_shell_path` (GUI PATH → `None`, candidate
search → found, non-executable → rejected) and `the_version_probe_gets_a_path_that_can_find_node`.

---

## Gates
```
cargo test                        233 passed, 0 failed, 3 ignored (live network, opt-in)
cargo clippy -D warnings, fmt      clean
npx vitest run                    124 passed / 14 files
lint, prettier --check, tsc        clean
scripts/dep-audit.sh               PASS (smaller graph: keyring removed)
```
