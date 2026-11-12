# V3 — Release limitations

Everything here is a limitation of the shipped build, stated so nothing is
mistaken for a supported capability.

## Not implemented

| Limitation | Consequence |
| --- | --- |
| No protocol bridge for OpenAI-only providers | NVIDIA NIM and similar endpoints are refused with `501 compatibility_error`. Use an Anthropic-compatible provider (Anthropic, OpenRouter, DeepSeek, Z.ai) or an external bridge. |
| Windows and Linux builds | Not produced or tested. The helper lifecycle is split into named Unix-specific functions (`launchctl`, `kill`, `ps`) so a port replaces those, not the design. |
| Apple Developer ID signing / notarization | The app is ad-hoc signed. First launch on another Mac needs right-click → Open, or `xattr -dr com.apple.quarantine`. |
| Ubuntu CI | Never executed — the GitHub account is billing-locked, so all three workflow jobs fail before starting. Local gates are the only evidence. |
| VoiceOver / contrast tooling | Not run. Accessibility is enforced structurally in tests, not audited with a screen reader or a contrast checker. |

## Behaviour worth knowing

- **Routed mode needs the helper.** Closing the app is fine; if the helper is
  stopped, the header says `mode: proxy stopped` and Claude Code cannot reach a
  provider until it is started again.
- **Login start is opt-in.** Without the LaunchAgent the helper lives for the
  current login session only.
- **Cooldowns are memory-only.** Restarting the helper forgets a 6-hour cooldown,
  so one extra attempt is possible — never a retry loop.
- **Only three headers are forwarded** besides the body (`anthropic-version`,
  `anthropic-beta`, `accept`).
- **`/v1/messages` only**, matched exactly; other paths return 404.
- **Request bodies are capped at 16 MiB**, headers at 64 KiB.
- **The health endpoint is on-demand**, not polled continuously.

## Not verified in this phase

- A live request against a real paid provider (needs the owner's key and money).
- A real Claude Code session through the proxy in a VS Code terminal, including a
  simulated upstream failure and recovery (V3-36) — the helper's survival and
  health were verified, the end-to-end Claude Code conversation was not.
- A reboot test. The login agent's plist is written and tested for content, but a
  real logout/login cycle was not performed.
- Clean-Mac install, Windows, Linux.
