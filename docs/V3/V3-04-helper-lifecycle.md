# V3-04 — Helper lifecycle design

**Requirement:** with routed mode on, Claude Code started from a VS Code terminal
must keep working after the configuration-manager window closes.

## What V2 did (and why it failed)

`AppState.proxy` held a `ProxyHandle` whose accept thread lived in the Tauri
process. `Drop for ProxyHandle` shut the listener down. Quitting the GUI
therefore closed the port, and Claude Code's next request failed. The docs even
said routed mode "needs this app running".

## Chosen architecture: the same binary, detached

```
Claude Code ──HTTP──▶ 127.0.0.1:<port>  ◀── listens ── helper process
   │                        │                            ▲
   └─ apiKeyHelper ──▶ ~/.claude-config-manager/helpers/proxy.sh
                                                         │ spawn (detached)
                                              GUI (Tauri) ──┘   … GUI may quit
```

- **One artifact.** The helper *is* the app binary started with
  `--proxy-helper`; `main()` branches before Tauri initialises. Nothing new to
  build, sign, ship or trust, and no second binary can drift out of date.
- **Detached.** Spawned with `process_group(0)` and `Stdio::null()`, so a signal
  to the app's process group (quit, Ctrl-C, force-quit) cannot reach it. On macOS
  it is reparented to `launchd` (ppid 1) the moment the GUI exits.
- **Identity-checked.** The helper writes `proxy-helper.json` (`0600`, atomic)
  with its pid and port; `/healthz` reports the same pid. The GUI **only ever
  signals a pid it can prove is answering on that port**, so a recycled pid can
  never be killed.
- **Stable across restarts.** The bearer token lives in
  `~/.claude-config-manager/secrets/proxy-helper.secret` (`0600`); the last port
  is remembered in `proxy-helper.port`. A restart therefore keeps both the port
  Claude Code is pointed at and the token its helper script returns — no
  settings rewrite, no broken session.
- **Crash recovery.** A killed helper leaves a state file that `status()` reports
  as `stale` (never `running`); *Start* or *Restart* recovers on the same port.
- **User-space only.** No administrator rights, no privileged helper, no
  system-wide daemon, no kernel extensions.

## Login start is an explicit opt-in

`helper_login_start(true)` writes
`~/Library/LaunchAgents/io.github.shahid8142.claude-config-manager.proxy.plist`
(`RunAtLoad`, `KeepAlive.SuccessfulExit=false`, `ProcessType Background`, log in
`~/.claude-config-manager/proxy-helper.log`) and bootstraps it into
`gui/<uid>`. `helper_login_start(false)` boots it out and deletes the file. The
app never calls this on its own: the Failover screen shows a checkbox that says
what it installs and where.

Without the agent, the helper still survives GUI exit for the rest of the login
session — which is what the VS Code terminal scenario needs — and dies at logout.

## Boundaries kept for portability

Windows has no `launchctl` and no `kill`; the portable surface is therefore:
state file + `/healthz` identity + spawn/terminate behind
`helper::start_with_exe` / `helper::stop`. Only `install_login_agent` and
`stop`'s signal call are macOS/Unix-specific, and both are isolated in named
functions so a Windows implementation replaces two functions, not the design.

## Deliberate non-choices

| Option | Why not |
| --- | --- |
| Keep the helper a child of the GUI and hope it survives | V2's failure mode; `Drop` is the documented behaviour |
| LaunchAgent installed automatically | Silent persistent service — explicitly forbidden |
| Separate `ccm-proxy` binary | Second artifact to sign/ship; can drift from the GUI's logic |
| Rebind a random port each start | Would invalidate the applied `ANTHROPIC_BASE_URL` |
| Rotate the bearer token every restart | Would invalidate the applied `apiKeyHelper` |

## Verification

`src-tauri/tests/helper_lifecycle.rs` (real binary, 5 tests): detached start +
`/healthz` + own process group, idempotent start, token/port reuse across
restart, SIGKILL → `stale` → restart recovery, and `stop` refusing to signal a
pid it cannot verify.

Manual end-to-end: helper launched from a shell that then exited answered
`/healthz` on its port with `ppid 1`; `SIGTERM` closed the port.
