# Claude Config Manager

A macOS desktop app that configures the **Claude Code CLI** without shell commands — pick a provider, store your key safely, choose a model, and apply with a verified, reversible transaction.

![platform](https://img.shields.io/badge/platform-macOS%2014%2B-blue) ![license](https://img.shields.io/badge/license-MIT-green) ![tests](https://img.shields.io/badge/tests-450%2B-brightgreen)

Built with Tauri 2 · Rust · React · TypeScript · Tailwind CSS.

## What it does

- **Live truth** — reads `~/.claude/settings.json` and shows exactly what Claude Code is using right now (model, endpoint, key mode), with override warnings when your shell environment wins.
- **Safe apply** — every apply is a transaction: backup → atomic write → read-back verification → rollback on mismatch. A broken settings file is never overwritten.
- **Credentials** — multiple keys per provider, stored as owner-only files (`0600`) handed to the CLI via an `apiKeyHelper` script. Keys never appear in `settings.json`, logs, or errors.
- **Providers** — verified presets (Anthropic, OpenRouter, DeepSeek, Z.AI, NVIDIA NIM) plus custom Anthropic-compatible endpoints. Model catalogs fetched from documented endpoints; nothing is ever auto-selected.
- **Failover** (optional) — a loopback-only proxy that tries your targets in order; survives GUI closure via a detached helper.
- **Bridge** (optional, consent-gated) — a pinned, checksum-verified third-party translator for OpenAI-only providers. Nothing downloads or executes without your explicit consent.
- **Maintenance** — environment checks, backups list, one-click restore (itself reversible).

## Install

1. Download `Claude Config Manager_1.0.0_aarch64.dmg` from the release.
2. Open it and drag **Claude Config Manager** to **Applications**.
3. Launch. Because the build is **unsigned**, Gatekeeper will warn: right-click → **Open** → **Open** (once), or run:

   ```sh
   xattr -dr com.apple.quarantine "/Applications/Claude Config Manager.app"
   ```

   (Removing quarantine is only needed for unsigned local builds — signed releases won't need it.)

## Security model

- Credentials live in `~/Library/Application Support/ClaudeConfigManager/secrets/` — directory `0700`, files `0600`, written atomically. The index beside them holds labels and order only, never key material.
- The proxy and bridge bind **127.0.0.1 only**. The proxy requires a bearer token; the bridge reads keys from files, never config.
- No telemetry, no cloud, no background anything without an explicit toggle (the failover helper is opt-in per login).

## Build from source

```sh
npm install
npm run tauri dev      # develop
npm run tauri build    # produce .app + .dmg in src-tauri/target/release/bundle/
```

Requires Node ≥ 22 and Rust (stable). Linux/Windows builds are untested — macOS is the supported target.

## Tests

```sh
npm run check:frontend   # eslint + prettier + tsc + vitest
npm run check:rust       # fmt + clippy -D warnings + cargo test (13 suites)
```

Per-feature evidence lives in `docs/audits/`; design system in `DESIGN.md`; threat model in `docs/security/`.

## License

[MIT](LICENSE)
