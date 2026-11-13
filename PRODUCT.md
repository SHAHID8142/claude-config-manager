# PRODUCT.md — Claude Config Manager

*(Authored 2026-11-13 by the coding agent from `01-PRD.md` + owner messages; the owner delegated design decisions in brief: "cleaner, less text, more polished, professional, animated, perfect".)*

## What this is

A macOS-first desktop utility (Tauri 2 · Rust · React · TypeScript · Tailwind) that configures the **Claude Code CLI's global settings** without shell commands. Windows-ready architecture; not a Claude Desktop replacement; no per-project config.

## Who uses it

Developers who run Claude Code inside VS Code's integrated terminal. They are technical, comfortable with concepts (API keys, endpoints, model ids), and impatient with hand-holding — but the *consequences* (a corrupted `~/.claude/settings.json`, a leaked key) are ones they care about deeply.

## Jobs the product does (in order of importance)

1. **Show the truth**: what is Claude Code using right now (model, endpoint, key mode) — read-only, live.
2. **Change it safely**: provider → credential → model → Apply, as a transaction (backup → atomic write → verify → rollback). Never overwrites a broken file. Never leaks the key.
3. **Store keys** as owner-only files (0600) handed to the CLI via an `apiKeyHelper` script; multiple keys per provider with priority.
4. **Optional failover**: a loopback-only proxy that tries targets in order; survives GUI closure via a detached helper.
5. **Maintain**: environment checks, backups list, restore, honest diagnostics.

## Hard product rules (never violated by design)

- Never auto-select a model; selection is always explicit.
- Never fabricate status or capability ("unknown" is a first-class state).
- Never display the API key in full after entry; masked + reveal only.
- The key never appears in `settings.json`, logs, or errors.
- Apply is explicit — it is a transaction, not a preference save.

## Personality

Precise, calm, trustworthy. A well-made instrument, not a marketing site. Copy is short, declarative, and speaks in the user's language. Zero cheerleading.
