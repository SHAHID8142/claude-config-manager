# V3-01 — Repository baseline

**Date:** 2026-11-08 · **Branch:** `main` · **Remote:** `origin` = https://github.com/SHAHID8142/claude-config-manager.git

## Git state at start

```
9d17575 docs(v2): record the two owner-requested UI passes in the tracker
47f40b2 feat(ui): macOS sidebar–detail shell with one screen per concern
2811e46 feat(ui): single-column settings layout with the configuration summary first
```

`git status --porcelain` empty. Local `HEAD` == `origin/main` (`9d17575`).

## Executed gates (this machine, macOS arm64)

| Command | Result |
| --- | --- |
| `cargo test --manifest-path src-tauri/Cargo.toml` | exit 0 — **291 passed, 0 failed** (10 test binaries; 3 live tests ignored by design) |
| `npx vitest run` | exit 0 — **185 passed / 24 files** |
| `npx tsc --noEmit` | clean |
| `npm run lint` (`eslint .`) | clean |
| `npm run tauri build` | exit 0 — app bundle + DMG produced |

Toolchain: node v26.10.0, cargo 1.99.1, gh 2.102.0.

## Documentation vs implementation — discrepancies found

1. **`FINAL_AUDIT.md` / `docs/V2/V2-42-verification-report.md` describe CI as
   healthy.** It is not: every workflow run since V2 has failed at ~3 s because
   the GitHub account is billing-locked, so no job ever starts. No code failure
   is involved. Recorded in `V3-02-defects.md` §7; cannot be fixed from the repo.
2. **`docs/USER_GUIDE.md` §11 says routed mode "needs this app running".** That
   is accurate for V2 but is exactly the V3 defect; Phase B replaces it.
3. **`docs/V2/V2-TRACKER.md` V2-51 is `partial`** (clean-Mac, Windows, Linux
   unverified). Still true and unchanged by V3.
4. **`src/tabs/CredentialsTab.tsx` claims the key directory is
   `~/Library/Application Support/com.claude-config-manager/secrets/`** while the
   backend uses `~/.claude-config-manager/secrets/`. Confirmed defect (V3-26).
5. **The V2 helper script path** is `~/.claude-config-manager/helpers/<provider>.sh`
   (space-free by design, G16) — documentation and code agree here.

## Branch policy

All V3 work lands on `main`, one commit per task, pushed after gates pass.
Nothing was force-pushed, rebased or amended; the V2 history is intact.
