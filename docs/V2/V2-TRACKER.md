# Claude Config Manager — V2 Tracker

Recoverable state for the V2 phase. One row per atomic task. **A task is only `done` when its
evidence table is filled in and the commit is pushed.**

Baseline (start of V2): `e6e6bf8` · Rust 246 passed / 0 failed · TS 130 passed / 14 files.

| Task | Title | Status | Commit |
| --- | --- | --- | --- |
| V2-01 | Inspect repo, branch, tree, baseline tests | done | `6f465be` |
| V2-02 | Document UI + credential-storage limitations | done | `ee9a9d4` |
| V2-03 | Verify Claude Code config + router compatibility | done | `7fd266e` |
| V2-04 | V2 tracker, acceptance criteria, migration strategy | done | `ab14026` |
| V2-05 | Design tokens (spacing, type, color, border, surface) | done | `1ee5e07` |
| V2-06 | Native-feeling window layout and sizing | done | `039e650` |
| V2-07 | Header + active configuration summary | done | `fff8c83` |
| V2-08 | Reusable icon-button component | done | `d278306` |
| V2-09 | Accessible tooltip + help popover | done | `1b9b45a` |
| V2-10 | Input, dropdown, button, switch restyle | done | `addd480` |
| V2-11 | Compact form layout, responsive behavior | done | `81267e2` |
| V2-12 | Simplified provider + credential UI | done | `2c0a9b6` + `2a21665` |
| V2-13 | Simplified model selection UI | done | `699652e` |
| V2-14 | Status / loading / error / confirmation feedback | done | `39e8752` |
| V2-15 | Advanced settings disclosure | done | `3a8c7f1` |
| V2-16 | Light/dark themes + accessibility verification | done | `d2bfe14` + `4a93f0d` |
| V2-17 | App icon integration + validation | done | `2966a0c` |
| V2-18 | Credential-ID + provider-profile schemas | done | `ef3e17e` |
| V2-19 | Backward-compatible credential store migration | done | `cd32510` |
| V2-20 | Multiple credentials per provider (backend) | done | `4fc37fe` |
| V2-21 | Credential management + masked selection UI | done | `a3ed54c` |
| V2-22 | Credential priority + enable/disable | done | `docs/V2/V2-22-priority.md` |
| V2-23 | Multiple routing targets | done | `3597a19` |
| V2-24 | Compact priority editor | done | `3597a19` |
| V2-25 | Migration/permission/redaction/isolation tests | done | `3597a19` |
| V2-26 | Router decision: own Rust proxy (ADR-003) | done | `docs/decisions/ADR-003` |
| V2-27 | Third-party consent + download verification | n/a — nothing is downloaded | `3597a19` |
| V2-28 | Router config generation | done | `3597a19` |
| V2-29 | Loopback routing + startup | done | `3597a19` |
| V2-30 | Helper shutdown/restart/recovery | done | `3597a19` |
| V2-31 | Routing endpoint into Claude Code config | done | `3597a19` |
| V2-32 | Routing health checks | partial — on-demand status only | `3597a19` |
| V2-33 | Provider error-classification rules | done | `3597a19` |
| V2-34 | Rate-limit / quota-exhaustion detection | done | `3597a19` |
| V2-35 | Bounded retry + cooldown | done | `3597a19` |
| V2-36 | Ordered credential/model fallback | done | `3597a19` |
| V2-37 | Cross-provider fallback | done | `3597a19` |
| V2-38 | Auto Failover UI toggle | done | `3597a19` |
| V2-39 | Routing status + safe diagnostics | done | `3597a19` |
| V2-40 | Exhausted/invalid/timeout/5xx tests | done | `3597a19` |
| V2-41 | Streaming, tools, replay safety, recovery | done | `3597a19` |
| V2-42 | Direct-mode workflow test | todo | — |
| V2-43 | Routed-mode workflow test | todo | — |
| V2-44 | Credential/migration/localhost/log audit | todo | — |
| V2-45 | Dependency + supply-chain audit | todo | — |
| V2-46 | Keyboard, a11y, sizing, visual regression | todo | — |
| V2-47 | Claude Code config safety + reversibility | todo | — |
| V2-48 | Close/reopen, helper lifecycle, uninstall | todo | — |
| V2-49 | Safe cleanup of project build caches | todo | — |
| V2-50 | Full gate run (fmt, lint, types, tests, build) | todo | — |
| V2-51 | macOS smoke test + unverified envs | todo | — |
| V2-52 | User docs + limitations | todo | — |
| V2-53 | Final audit report | todo | — |
| V2-54 | Commit, push, deliver | todo | — |
