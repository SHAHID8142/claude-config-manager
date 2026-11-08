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
| V2-11 | Compact form layout, responsive behavior | done | committed |
| V2-12 | Simplified provider + credential UI | todo | — |
| V2-13 | Simplified model selection UI | todo | — |
| V2-14 | Status / loading / error / confirmation feedback | todo | — |
| V2-15 | Advanced settings disclosure | done | next |
| V2-16 | Light/dark themes + accessibility verification | todo | — |
| V2-17 | App icon integration + validation | todo | — |
| V2-18 | Credential-ID + provider-profile schemas | done | `docs/V2/V2-18-credential-schema.md` |
| V2-19 | Backward-compatible credential store migration | done | `docs/V2/V2-19-migration.md` |
| V2-20 | Multiple credentials per provider (backend) | done | `docs/V2/V2-20-multi-credential-vault.md` |
| V2-21 | Credential management + masked selection UI | todo | — |
| V2-22 | Credential priority + enable/disable | todo | — |
| V2-23 | Multiple routing targets | todo | — |
| V2-24 | Compact priority editor | todo | — |
| V2-25 | Migration/permission/redaction/isolation tests | todo | — |
| V2-26 | Verify router release + integration contract | todo | — |
| V2-27 | Third-party consent + pinned download verification | todo | — |
| V2-28 | Router config generation | todo | — |
| V2-29 | Loopback routing + startup | todo | — |
| V2-30 | Helper shutdown/restart/recovery/GUI-close | todo | — |
| V2-31 | Routing endpoint into Claude Code config | todo | — |
| V2-32 | Routing health checks | todo | — |
| V2-33 | Provider error-classification rules | todo | — |
| V2-34 | Rate-limit / quota-exhaustion detection | todo | — |
| V2-35 | Bounded retry + cooldown | todo | — |
| V2-36 | Ordered credential/model fallback | todo | — |
| V2-37 | Cross-provider fallback | todo | — |
| V2-38 | Auto Failover UI toggle | todo | — |
| V2-39 | Routing status + safe diagnostics | todo | — |
| V2-40 | Exhausted/invalid/timeout/5xx tests | todo | — |
| V2-41 | Streaming, tools, replay safety, recovery | todo | — |
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
