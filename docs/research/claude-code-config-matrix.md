# Claude Code Global Configuration — Verified Matrix (T02)

**Research date:** 2026-11-08 · **Status:** VERIFIED against official documentation (URLs below) + live local-machine cross-check. This document is the binding reference for what our app may write.

**Verification method:** official docs fetched from `code.claude.com/docs` (docs.anthropic.com redirects there, 301 confirmed) + read-only inspection of this machine's `~/.claude/` (all secret values redacted before recording). Re-verify at implementation time of any config-writing task (rule 9).

## 1. Settings file locations and precedence (official)

"Claude Code reads settings from four files, and an organization can also deliver managed settings from the claude.ai console."

| Priority | Source | Path |
|---|---|---|
| 1 (highest) | Managed settings | MDM/OS policies or server-managed from claude.ai console — "Nothing in your own settings files or `--settings` overrides a managed key" |
| 2 | Command line | `claude --settings <file-or-json>` (session-only; cannot set managed/global keys) |
| 3 | Project local | `<project>/.claude/settings.local.json` |
| 4 | Shared project | `<project>/.claude/settings.json` |
| 5 | User (global) | `~/.claude/settings.json` ← **our app's only write target** |

Verbatim: "managed settings, command line, project local, shared project, user. A key set at a higher level overrides the same key set lower down."

Other facts (official):
- `~/.claude.json` — written by Claude Code itself (account details, MCP servers, per-project trust, `/config` global keys). **Never write this file.**
- `CLAUDE_CONFIG_DIR` relocates settings/history/plugins (incl. the macOS credentials fallback file). Our app must honor it when *reading*, never set it.
- Files are **strict JSON**: `//` comments and trailing commas are syntax errors ("Settings Error at next start"). → our merge must emit strict JSON only.
- Lists merge across files instead of overriding (e.g. `permissions.allow`) — exceptions: `fallbackModel`, `modelPicker`, `availableModels`, `modelSettings` (managed applies as-is).
- `/status` shows loaded "Setting sources"; `claude doctor` lists rejected entries. Useful for read-back verification (T34).

## 2. Authentication mechanisms and precedence (official, verbatim order)

When multiple credentials exist, Claude Code picks in this order:
1. **Cloud provider credentials** — when `CLAUDE_CODE_USE_BEDROCK` / `CLAUDE_CODE_USE_VERTEX` / `CLAUDE_CODE_USE_FOUNDRY` is set
2. **`ANTHROPIC_AUTH_TOKEN`** — "Custom value for the `Authorization` header (…prefixed with `Bearer `)" — for gateways/proxies with bearer auth
3. **`ANTHROPIC_API_KEY`** — "API key sent as `X-Api-Key` header"; used over subscription even when logged in; non-interactive always uses when present, interactive prompts once for approval
4. **`apiKeyHelper`** script output
5. **`CLAUDE_CODE_OAUTH_TOKEN`** — one-year token via `claude setup-token` (CI)
6. Anthropic profile / federation credentials
7. **Subscription OAuth from `/login`** — default for Pro/Max/Ultra/Team/Enterprise

Overrides: a signed-in Claude apps gateway session outranks everything; managed `forceLoginMethod: "gateway"` forces gateway sign-in and **blocks** sessions authenticating via `apiKeyHelper`/`ANTHROPIC_API_KEY`/`ANTHROPIC_AUTH_TOKEN`.

Credential storage (official):
- **macOS:** encrypted Keychain; if Keychain write is refused, falls back to `~/.claude/.credentials.json` mode `0600`
- **Linux:** `~/.claude/.credentials.json` mode `0600`
- **Windows:** `%USERPROFILE%\.claude\.credentials.json`

## 3. `apiKeyHelper` (official)

- Settings key running "a shell script that returns an API key" — for "dynamic or rotating credentials, such as short-lived tokens fetched from a vault".
- Hot-reloads on file change; re-run cadence is in the settings reference — **must be re-verified in T32 before implementing handoff**.
- >10 s runtime → warning notice in prompt bar. Failure (error exit, timeout, empty output) → "apiKeyHelper script is failing" error within three attempts.
- Applies to CLI + VS Code extension + Agent SDK; **not** Claude Desktop or cloud sessions.

## 4. Environment variables relevant to our app (official `env-vars` reference)

**Auth/routing:**
| Var | Meaning |
|---|---|
| `ANTHROPIC_API_KEY` | X-Api-Key header |
| `ANTHROPIC_AUTH_TOKEN` | Authorization: Bearer header (gateway/proxy) |
| `ANTHROPIC_BASE_URL` | "Override the API endpoint to route requests through a proxy or gateway." Since v2.1.196 Remote Control is disabled when it ≠ `api.anthropic.com`. |
| `ANTHROPIC_CUSTOM_HEADERS` | Extra `Name: Value` headers, newline-separated (v2.1.227+) |
| `API_TIMEOUT_MS` | per-attempt response-header timeout; default 600000 |

**Model selection:**
| Var | Meaning |
|---|---|
| `ANTHROPIC_MODEL` | overrides `model` key from any files; read first |
| `ANTHROPIC_DEFAULT_HAIKU_MODEL` | `haiku` alias target, "also used for background functionality" |
| `ANTHROPIC_DEFAULT_SONNET_MODEL` | `sonnet` alias target; `opusplan` outside Plan Mode |
| `ANTHROPIC_DEFAULT_OPUS_MODEL` | `opus` alias target; `opusplan` in Plan Mode |
| `ANTHROPIC_SMALL_FAST_MODEL` | **[DEPRECATED]** Haiku-class for background tasks |
| `ANTHROPIC_DEFAULT_MODEL` | default for new sessions only when no file sets `model` (v2.1.236+) |
| each `*_NAME` / `*_DESCRIPTION` / `*_SUPPORTED_CAPABILITIES` | `/model` picker display metadata |
| `CLAUDE_CODE_MAX_OUTPUT_TOKENS` | max output tokens for most requests |

**Precedence pairs (verbatim):** `--model`/`/model` override `ANTHROPIC_MODEL`; `CLAUDE_CODE_EFFORT_LEVEL` overrides `--effort`/`/effort`; `ANTHROPIC_MODEL` overrides `model` from any file; `ANTHROPIC_DEFAULT_MODEL` applies only if no file sets `model`.

## 5. `env` block semantics in settings.json (official — critical for our writer)

- "Claude Code copies these values into its environment as written. **No shell processes them**" — `~` and `$HOME` are **not expanded** → absolute paths required (relevant if we ever write an `apiKeyHelper` path).
- Settings-file value wins over shell value "in most sessions".
- Settings can set but **not remove** a variable; writing `""` (empty string) treats it as unset for provider selection.
- Project/local files cannot set some variables (e.g. `CLAUDE_CONFIG_DIR`, OTEL exporters) — we only write user scope, so this doesn't restrict us, but we must not write ignored keys.

## 6. Documented settings.json keys — our supported write set

Our app writes **only** keys verified below; every other key in the user file is preserved untouched (F02, T27).

| Key | Verified meaning | Our use |
|---|---|---|
| `env.ANTHROPIC_BASE_URL` | endpoint override | set for provider/bridge routing |
| `env.ANTHROPIC_AUTH_TOKEN` | bearer token | gateway-style providers (note: plaintext in settings file — see §8) |
| `env.ANTHROPIC_API_KEY` | X-Api-Key | Anthropic-direct (plaintext — see §8) |
| `env.ANTHROPIC_MODEL` | model override | set when user picks a model |
| `env.ANTHROPIC_DEFAULT_HAIKU/SONNET/OPUS_MODEL` | role aliases | advanced panel only, verified keys, default untouched |
| `model` | starting model | set when user picks a model |
| `apiKeyHelper` | script returning API key | secure handoff path (T32) |
| `$schema` | JSON-schema hint (`https://json.schemastore.org/claude-code-settings.json`) | preserve if present |

Observed on this machine but **not** treated as our write set (version-specific/undocumented for us): `CLAUDE_CODE_ENABLE_GATEWAY_MODEL_DISCOVERY`, `CLAUDE_CODE_AUTO_COMPACT_WINDOW`, `modelSettings`, `skillOverrides`, `statusLine`, `enabledPlugins`, `extraKnownMarketplaces`, `skipDangerousModePermissionPrompt`, `theme`, `permissions.*` — all preserved as unrelated keys.

## 7. Local machine cross-check (2026-11-08, read-only, redacted)

- `~/.claude/settings.json` exists, valid JSON, contains `env` block with `ANTHROPIC_BASE_URL: http://127.0.0.1:31415`, `ANTHROPIC_AUTH_TOKEN` and `ANTHROPIC_API_KEY` set (values redacted), `ANTHROPIC_MODEL`/`ANTHROPIC_DEFAULT_*_MODEL` set to `"auto"`.
- Confirms the `env`-block pattern is actively used on real installs, and that a pre-existing local proxy may already occupy the config → **our app must detect/report pre-existing `env` values and warn before overwriting** (T34 override warnings; never silently clobber).
- `~/.claude/` also contains many state dirs (backups/, plugins/, history.jsonl …) — none of which we touch.

## 8. Security implications recorded for later tasks

1. **Plaintext tradeoff:** `env.ANTHROPIC_API_KEY`/`ANTHROPIC_AUTH_TOKEN` sit in plaintext JSON. Keychain alone is insufficient if CLI can't read it (F08). Options: (a) plaintext env with disclosed tradeoff + 0600 file + backups redacted/encrypted, (b) `apiKeyHelper` script reading from macOS Keychain — keeps the secret out of settings.json but is a script the CLI executes. Decision + consent flow happens in **T32**; STOP if unresolved.
2. **Precedence surprise risk:** setting `ANTHROPIC_AUTH_TOKEN` silently outranks subscription OAuth — our Save & Apply must warn that applying a token switches auth away from logged-in subscription (F06 "detect likely override conditions and report uncertainty").
3. **Strict JSON** on write; preserve key order/whitespace as far as possible (diff-friendly).
4. `ANTHROPIC_BASE_URL` pointing anywhere ≠ api.anthropic.com disables Remote Control (v2.1.196+) — disclose when applying bridge/custom endpoints.

## 9. Sources (fetched 2026-11-08)

- Settings & precedence: https://code.claude.com/docs/en/settings (301 from docs.anthropic.com/en/docs/claude-code/settings)
- Environment variables: https://code.claude.com/docs/en/env-vars
- Authentication: https://code.claude.com/docs/en/authentication
- Support: https://support.claude.com/en/articles/12304248-manage-api-key-environment-variables-in-claude-code
- Live cross-check: local `~/.claude/settings.json` (redacted inspection, this machine)
