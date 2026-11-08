# G16 — `apiKeyHelper` path contained a space ⇒ every request failed (exit 127)

**Trigger:** owner ran `claude` and `/status` reported:

```
apiKeyHelper: Failing — last run exited 127:
/bin/sh: /Users/maryamahad/Library/Application: No such file or directory
```

## Root cause
Helpers were installed under `~/Library/Application Support/ClaudeConfigManager/helpers/`.
Claude Code executes the `apiKeyHelper` value **as-is** (not through a quoted shell
invocation), so the path was cut at the space: `…/Library/Application`. Every request
then failed authentication even though the credential file and the script were valid.
My own tests missed it because fixtures used space-free temp directories and the script
was invoked by path (`Command::new(script)`), never through the value Claude Code reads.

## Fix
1. App-owned data moved to a **space-free** root: `~/.claude-config-manager/`
   (`helpers/`, `secrets/`). Backups stay under Application Support — nothing executes them.
2. `DesiredSettings::validate` now **rejects any whitespace** in an `apiKeyHelper` path, so a
   future relocation cannot silently reintroduce this.
3. The generated script prints its own path in the "missing" diagnostic, so a wrong path is
   visible in one line instead of a bare 127.

## Verification
- `helper_path_with_a_space_is_refused` (new), plus the existing lifecycle tests that execute
  the **real generated script** from a fixture HOME — they now run against
  `~/.claude-config-manager/` and still print exactly the credential.
- `script_is_fixed_shape_with_validated_id` asserts the script contains
  `.claude-config-manager/secrets/anthropic.secret` **and** no `Application Support`.
- End-to-end after reinstall: `sh ~/.claude-config-manager/helpers/<id>.sh` prints the key and
  `claude` authenticates.

## Follow-up false positive found on the fresh install
The first fresh launch still warned "ANTHROPIC_AUTH_TOKEN (36 chars)" — because the probe shell
**inherited** the variable from the terminal that launched the app, and the shell snippet could not tell
"exported but empty" from "unset". Both fixed: the probe runs with a cleared environment (HOME/USER/SHELL/
TERM/PATH only) and uses `printenv` to distinguish set-empty from unset. A process that inherits a stale
token now reports `shell_env: []` unless the user's shell **files** actually export it.

## Lesson recorded
Every path that gets **executed by a third-party tool** must be treated as untrusted input for
quoting purposes: verify it by running exactly what the tool runs (here: the unquoted path), not
by calling the file ourselves.
