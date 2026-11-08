# G15c — a shell export silently overrode everything the app wrote

**Trigger:** owner screenshots — the app showed "Configuration applied and verified" with
`model: stealth/crouton-alpha`, while Claude Code printed:

```
⚠ Both ANTHROPIC_AUTH_TOKEN and apiKeyHelper set · auth may not work as expected
✱ 401 Missing Authentication header · Retrying in 15s · attempt 7/10
```

## What was actually true
The saved file was **correct** (checked read-only):

```
apiKeyHelper: .../helpers/openrouter.sh
model: stealth/crouton-alpha
ANTHROPIC_AUTH_TOKEN / API_KEY / MODEL: absent
ANTHROPIC_BASE_URL: https://openrouter.ai/api        (25 chars)
```

The conflict came from the login shell:

```
~/.zshrc:  export ATRIA_API_KEY="…"
           export ANTHROPIC_AUTH_TOKEN="$ATRIA_API_KEY"   # 36 chars, prefix atr_N…
```

The stored OpenRouter key is `sk-or-v1…` (73 chars) and the helper prints it correctly. Claude Code ranks
`ANTHROPIC_AUTH_TOKEN` **above** `apiKeyHelper`, so the wrong 36-char token went to OpenRouter → 401.

A GUI app launched from Finder **cannot see the shell environment** — it inherits launchd's, not `~/.zshrc`.
So the app had no way to know, and truthfully reported only what it could see.

## Fix — detect it instead of asserting it cannot happen
`environment_check` now asks the user's **login shell** (`zsh -lic`, then `bash -lic`, 3 s cap) which
shadowable variables it exports, and reports **names and lengths only**:

```
shell_env: [ANTHROPIC_AUTH_TOKEN=36, ANTHROPIC_API_KEY=0, ANTHROPIC_BASE_URL=0, ANTHROPIC_MODEL=0]
```

Values are never requested, so no credential can reach the UI, a log, or an error. The UI shows a warning:
*"Your shell exports ANTHROPIC settings that override this app"* with the variable names and lengths, and the
user guide's troubleshooting table now maps that exact Claude Code 401 message to this cause.

Tests: `shell_exports_are_reported_by_name_and_length_only` (values are not even parsed),
`no_exports_produce_an_empty_report`, plus 2 UI tests (warning shown with a 36-char export, silence when
nothing is exported).

## Honest limits
The probe runs a login shell, so a pathological `~/.zshrc` can slow it (bounded to 3 s and it is a
diagnostic only). If the shell exports a variable only in interactive use, detection can miss it — the UI
wording says "login shell" for that reason.

## Gates
```
cargo test    246 passed, 0 failed    clippy -D warnings, fmt   clean
vitest        130 passed / 14 files   lint, prettier, tsc       clean
```
