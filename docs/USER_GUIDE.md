# Claude Config Manager — User Guide

A macOS desktop app that configures the **Claude Code CLI** globally: pick a provider, store its key safely,
choose a model, and write `settings.json` safely (with backups and rollback) — without hand-editing JSON.

> Status: the app builds and its logic is heavily tested, but **the packaged build is unsigned and not
> notarized**, so it cannot be installed on a clean Mac yet. See [Limitations](#limitations) and
> `docs/audits/T64-package.md`.

---

## 1. What it changes (and what it never touches)

The app manages exactly these keys in your Claude Code settings file
(`~/.claude/settings.json`, or `$CLAUDE_CONFIG_DIR/settings.json`):

| Key | Meaning |
|---|---|
| `model` | The model Claude Code uses |
| `apiKeyHelper` | Path to a small script that prints your key from the Keychain |
| `env.ANTHROPIC_BASE_URL` | Provider endpoint |
| `env.ANTHROPIC_DEFAULT_HAIKU_MODEL` / `..._SONNET_MODEL` / `..._OPUS_MODEL` | Optional per-role model overrides |

Everything else in the file — permissions, hooks, status line, your own env vars — is preserved byte-for-byte
by the merge and verified by read-back. It **never** deletes a key it did not write, and it **never**
overwrites a file it cannot parse.

---

## 2. First run

1. Launch the app. The **Active configuration** panel shows what Claude Code currently uses
   (`not configured` on a fresh machine).
2. Choose a **Provider**.
   - Anthropic (native), OpenRouter, DeepSeek, Z.AI and NVIDIA NIM ship as verified presets.
   - **Custom** lets you type your own endpoint (https only).
3. Paste your **API key**. It is stored in the **macOS Keychain** (service
   `com.claude-config-manager.<provider>`), never written to disk. The field is masked and can be revealed
   only deliberately.
4. Optionally press **Fetch models** to load the provider's catalog. Providers without a documented catalog
   (currently Z.AI) ask you to type a model id instead.
5. Pick or type a **Model**.
6. Press **Save & Apply**.

Apply is fail-closed and transactional: validate → store credential → install helper script → snapshot →
merge → backup → atomic write → read-back → report. If any step fails, your previous configuration is intact.

---

## 3. Reading the result

- **Success banner** shows the backup file name and the helper path.
- **Active configuration** re-reads the file afterwards, so what you see is what is on disk.
- **Warnings** (never hidden):
  - *Helper shadowed by env* — you have plaintext `ANTHROPIC_AUTH_TOKEN`/`ANTHROPIC_API_KEY` in `env`, which
    outranks the helper.
  - *Plaintext key in env* — the app discloses that a key sits in the file in clear text.
  - *Model conflict* — `model` and `env.ANTHROPIC_MODEL` disagree; the env value wins in Claude Code.

---

## 4. Model roles (Advanced settings)

Collapsed by default. Each field is **optional** and blank means *no override* — Claude Code's own default
applies. The panel always shows the value that is currently configured, so it can never imply "not set" for
something that is set.

- The app never invents a mapping and never falls back to a different model
  (`docs/decisions/ADR-002-no-model-failover.md`).
- An invalid id (spaces, control characters, > 256 chars, or anything that looks like an API key) blocks
  Save & Apply with an explanation.
- **Clearing an override back to "no override" is not possible from the panel** — the merge contract only
  sets keys. Edit the file or restore a backup.

---

## 5. Backup and restore

Every apply that finds an existing settings file first writes a timestamped backup next to it
(`~/Library/Application Support/ClaudeConfigManager/backups/`). **Restore last backup** replaces the current
file with the newest *valid* backup, and first saves the current state as a pre-restore copy, so a restore is
itself reversible.

A corrupted newest backup is refused before anything is written — you are told the line/column of the
problem instead of silently getting broken settings.

---

## 6. Uninstall

There is no uninstall button yet. To remove the app cleanly:

1. In `~/.claude/settings.json`, delete `model`, `apiKeyHelper`, and the `ANTHROPIC_*` keys inside `env` that
   this app manages. Keep everything else (permissions, hooks, your own env).
2. Delete `~/Library/Application Support/ClaudeConfigManager/helpers/`.
3. Delete `~/Library/Application Support/ClaudeConfigManager/backups/` **only if** you no longer want the
   ability to roll back.
4. Delete the Keychain item: `security delete-generic-password -s com.claude-config-manager.<provider> -a <provider>`.
5. Delete the app.

Then open a new terminal so Claude Code picks up the reverted configuration.

---

## 7. Troubleshooting

| Symptom | Likely cause | What to do |
|---|---|---|
| "Save & Apply" is greyed out | no provider, no key, invalid model id, or an invalid role override | read the red message under the field; it names the rule |
| `The settings file has a problem this app will not overwrite` | malformed JSON or the file is a symlink | fix the JSON by hand (the app refuses to guess); symlinks are refused for safety |
| Claude Code still uses the old endpoint | plaintext `ANTHROPIC_*` env vars in your shell or in `env` outrank the helper | the app warns about this; remove the stale variable |
| Claude Code says the model does not exist | the id is well-formed but not offered by that provider | the app validates *shape* only; check the provider's catalog |
| Helper returns nothing | Keychain item missing (deleted, or app reinstalled) | re-apply with your key, or `security find-generic-password -s com.claude-config-manager.<provider> -a <provider> -w` to check |
| "Fetch models" fails with 401 | the provider wants a different credential style for its catalog | DeepSeek already handled (Bearer); for others, type the model id manually |
| `…is damaged and can't be opened` | the build is unsigned (ad-hoc) | expected for this build; see Limitations |

Diagnostics never print your key. Error text is redacted by exact match **and** by shape (anything
credential-looking is replaced with `[REDACTED]`).

---

## 8. Limitations

- **Unsigned, not notarized build.** Gatekeeper refuses it; distribution requires an Apple Developer ID
  (`docs/audits/T64-package.md`). arm64 only.
- **Bridge mode (OpenAI-only providers) is not implemented.** Claude Code speaks the Anthropic API; providers
  such as NVIDIA NIM that only expose an OpenAI-compatible API need a translation bridge. Those tasks
  (T47–T58) are blocked pending a decision on installing a third-party binary, and the app makes no claim to
  support them. Selecting such a provider only writes configuration; requests would fail at Claude Code.
- **Model capability data is `unknown`.** The app does not claim a third-party model supports tool use,
  vision or reasoning — that depends on the model *and* on the provider's compatibility layer.
- **No automated uninstall**, no auto-update, no GUI-side "test connection" to the provider.
- **Clearing a role override** requires editing the file (see §4).

---

## 9. Bridge limitations (planned, not shipped)

If bridge support is ever approved, these are the constraints it must satisfy (already recorded as design
requirements, not as shipped features):

- local-only listener (loopback, never `0.0.0.0`), pinned version with integrity verification before
  execution, verified license/provenance, and an explicit first-run consent screen naming source, version and
  license;
- per-provider/per-model capability reporting, with anything unverified shown as unknown;
- an uninstall path, and a documented behaviour when the GUI is closed (who owns the bridge lifecycle);
- no silent stripping of tool calls, streaming, vision or reasoning.

None of this exists in the current build.

---

## 10. Windows porting notes

The architecture is deliberately tiered, so a Windows port is mostly a platform-adapter job:

| Layer | macOS today | Windows equivalent |
|---|---|---|
| Secret storage | Keychain via `security` CLI | Windows Credential Manager (`CredWrite`/`CredRead`, or the `keyring` crate's Windows backend) |
| Settings location | `~/.claude/settings.json` / `$CLAUDE_CONFIG_DIR` | `%USERPROFILE%\.claude\settings.json` (verify with the Claude Code docs before claiming parity) |
| Helper script | `/bin/sh` script calling `security` | `apiKeyHelper` on Windows is not covered by the test suite; a `.cmd`/`.ps1` equivalent would need its own security review (argument quoting, execution policy) |
| Atomic write | temp + `fsync` + `rename` | `MoveFileEx` with `MOVEFILE_REPLACE_EXISTING`; the "same directory temp file" rule still applies |
| File permissions | `0600` | ACLs (`icacls`) — the current tests assert POSIX modes and would need a platform-specific variant |
| Paths | `~/Library/Application Support/...` | `%LOCALAPPDATA%\ClaudeConfigManager` |

Also needed: CI on Windows, a Windows-aware `ConfigLocator`, and re-checking every `#[cfg(unix)]` test (the
whole filesystem suite is Unix-only today). Do **not** assume macOS behaviour transfers: the symlink refusals,
permission checks and helper execution model all differ.
