# V2-04 — Acceptance criteria and migration strategy

## 1. Acceptance criteria (each must be demonstrable, not asserted)

### A. UI
1. Window opens at a compact size (≤ 900×820) and **Save & Apply is visible without scrolling** at
   that size; resizing to the minimum size never clips a control.
2. Header shows app icon, title, live connection state, and a menu/settings affordance.
3. Active configuration is readable in one glance: provider, model, credential source, health.
4. Every explanatory paragraph replaced by a tooltip/popover that is reachable by **keyboard focus**
   and still announced to a screen reader (`aria-describedby`).
5. Light and dark both pass a contrast check on all text/controls (≥ 4.5:1 body, ≥ 3:1 large/UI).
6. Advanced settings collapsed by default; expandable via keyboard; state persists during the session.
7. Icons: Lucide set only, one stroke weight, consistent sizes; no emoji.
8. Icon: Dock/Finder/window show the owner's artwork, no clipping, correct at 16/32/128/512.
9. Empty states exist for: no provider, no credential, no models fetched, no failover targets.
10. All interactive elements reachable in DOM order with a visible focus ring.

### B. Multi-credential
11. Two credentials for the same provider can be stored simultaneously and both survive an app restart.
12. Credentials have id, label, provider, priority, enabled, created/updated; no plaintext secret in any
    metadata file or log.
13. If a credential file is missing/loosened/symlinked, that credential is reported unusable and **other
    credentials still work** (isolation).
14. Deleting a credential removes only its file; `apiKeyHelper` pointing to it is not left dangling (apply
    refuses / UI blocks with an explanation).
15. Migration from the current layout (`secrets/<provider>.secret`) is atomic, idempotent, reversible, and
    preserves the secret byte-for-byte (verified by hash before/after).

### C. Failover
16. Disabled by default. Enabling it requires explicit user action and shows exactly which targets are in order.
17. A 429 with `Retry-After` is retried under the cap then fails over; a 402/quota body fails over immediately;
    a 401 is **not** rotated forever (bounded, then surfaced as an auth error).
18. Cooldown: a failed target is skipped until its cooldown expires; the reason and the answering model are
    visible in the routing log.
19. All targets exhausted → one understandable error, no invented success.
20. Streaming responses are never replayed; a mid-stream failure surfaces as an error.
21. Tool-call bodies pass through unmodified; the proxy never rewrites tool results.
22. Proxy binds loopback only, requires a bearer token we generate, and refuses requests without it.
23. Killing the GUI leaves the proxy running in routed mode; quitting intentionally (Quit menu action) stops it.

### D. Release
24. `cargo fmt --check`, `clippy -D warnings`, `tsc`, `eslint`, `vitest`, `cargo test`, `npm run tauri build` all
    clean; test counts recorded.
25. No secret in repo/history/fixtures; `scripts/dep-audit.sh` passes.
26. Docs updated: user guide, limitations (Windows/Linux untested, ad-hoc signing), uninstall instructions.

## 2. Credential store migration (V2-19)

### Current layout (v1)
```
~/.claude-config-manager/
  helpers/<provider>.sh              # prints secrets/<provider>.secret
  secrets/<provider>.secret          # 0600, one per provider
```

### Target layout (v2)
```
~/.claude-config-manager/
  credentials.json                   # 0600, NO secrets: id, label, provider, priority, enabled, timestamps
  secrets/<credential-id>.secret     # 0600, id = [a-z0-9-]{1,64}
  helpers/<credential-id>.sh
```
`credential-id` = `<provider>-<slug>` (e.g. `openrouter-default`, `openrouter-work`).

### Migration algorithm (idempotent, atomic, reversible)
1. Read `credentials.json` if present; if it parses and `version == 1`, stop (already migrated).
2. List `secrets/*.secret` whose basename matches `^[a-z0-9-]{1,64}$` and is **not** already an id in the index.
3. For each: create credential `{ id: "<provider>-default", label: "Default", provider, priority: n,
   enabled: true }`, copy the secret to `secrets/<id>.secret` with `0600` via temp+fsync+rename, keep the
   original filename as a hard link target only if copy fails (never delete in the same step).
4. Write `credentials.json` via temp+fsync+rename. **Only after** the index is durable, remove the legacy
   `<provider>.secret` files (skippable with `KEEP_LEGACY=1`).
5. Re-running with a partial state must converge (no duplicate ids, no loss).

### Reversibility
`credentials.json` keeps `"legacy": {"<provider>": "<credential-id>"}` so a downgrade can restore
`secrets/<provider>.secret` by copying back. A CLI flag `--migrate-credentials=revert` performs it.
