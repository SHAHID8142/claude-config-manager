# V2-02 — Current UI and credential-storage limitations

Inspected: `src/App.tsx` (698 lines), `src/lib/ipc.ts` (221), `src/components/*`,
`src-tauri/src/file_secret_store.rs`, `credential_service.rs`, `secret_store.rs`,
`save_apply.rs`, `connection_test.rs`, `environment_check.rs`.

## 1. UI limitations (what makes it feel like a technical form)

| # | Limitation | Evidence | V2 fix |
| --- | --- | --- | --- |
| U1 | One 698-line component renders every section permanently expanded | `App.tsx` — 4 `<Section>`s, only Advanced collapses | V2-07/11/15 |
| U2 | No app chrome: title is an inline `<h1>`; no icon, no connection indicator, no menu/settings affordance | `App.tsx:269` | V2-07 |
| U3 | Explanations are paragraphs of prose (auth precedence, helper behaviour) instead of tooltips/popovers | Provider + Advanced sections | V2-09/12/15 |
| U4 | Native `<select>`/`<input>` styling only — no custom dropdown, switch, or segmented control | `App.tsx` form controls | V2-10 |
| U5 | No iconography anywhere (no Lucide, no affordance glyphs) | no `lucide-react` in `package.json` | V2-08 |
| U6 | Model entry is a fetch-then-list flow with no search, no metadata panel, no refresh state | `components/ModelList.tsx` | V2-13 |
| U7 | Feedback is a single banner; no inline field validation, no confirmations, no inline error per field | `components/StatusBanner.tsx` | V2-14 |
| U8 | Fixed footer keeps Save reachable but the page still needs scrolling at 880×600 | `App.tsx` fixed action row, `pb-28` | V2-06/11 |
| U9 | Dark mode is OS-driven only (`prefers-color-scheme`) — no in-app theme control, no verified contrast pass | `src/index.css:7` | V2-16 |
| U10 | No diagnostics/advanced view for technical detail; everything is on the main surface | — | V2-39 |
| U11 | No keyboard-focus affordances beyond the browser default; tooltips (once added) must be focusable | V2-09 | V2-16 |
| U12 | No empty states (no "no provider selected", "no models fetched" illustrations/copy) | Model section | V2-13/14 |

## 2. Credential-storage limitations (the blocker for multi-key failover)

| # | Limitation | Evidence | V2 fix |
| --- | --- | --- | --- |
| S1 | **Exactly one secret per provider.** Path is `<home>/.claude-config-manager/secrets/<provider>.secret`; a second OpenRouter key overwrites the first | `file_secret_store.rs` (`path_for` derives the filename from the account) | V2-18/19/20 |
| S2 | No credential metadata at all: no id, name, priority, enabled flag, created/updated time, last-used, or last-error | `credential_service.rs` — `set/has/delete(provider_id)` only | V2-18/22 |
| S3 | Credential identity is the **provider**, so two keys for one provider are unrepresentable — and the helper script (`apiKeyHelper`) can therefore only ever emit one key | `credential_handoff.rs` (`generate_helper_script`) | V2-18/20/28 |
| S4 | Claude Code accepts a **single** `apiKeyHelper` path and one `ANTHROPIC_BASE_URL`; there is no native rotation, so multi-key failover requires a localhost router | `config_merge.rs` managed keys | V2-26…V2-37 |
| S5 | Deletion is provider-scoped and immediate; there is no "disable" state, so removing a key to test another is destructive | `delete_credential` | V2-22 |
| S6 | No routing log or status surface exists, so a failover decision could not be explained to the user | — | V2-39 |
| S7 | Backups of `settings.json` live under `~/Library/Application Support/...` (space in path — fine for data, never executed) while secrets live under `~/.claude-config-manager/` | `config_backup.rs` vs `file_secret_store.rs` | keep, document |

## 3. What must NOT regress (preserve)

- `0600` file / `0700` dir creation at `open(2)` time (never chmod-after-write), symlink refusal both ways,
  loosened-mode refusal on read, atomic temp+fsync+rename, provider-component validation.
- Fail-closed apply pipeline and the existing backup/rollback path.
- Space-free helper path (G16) — `apiKeyHelper` is executed unquoted by Claude Code.
- No secret in any UI string, log, error, or fixture.
