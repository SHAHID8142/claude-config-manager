# T64 — macOS package: documented unsigned test build

**Date:** 2026-11-08 · **Result:** ✅ build produced and analysed · ⚠️ **NOT DISTRIBUTABLE** (unsigned,
not notarized, Gatekeeper refuses it) · **Blocker:** Apple Developer ID credentials (owner decision)

## What was built (verified locally)
| Artifact | Size | SHA-256 |
|---|---|---|
| `Claude Config Manager.app` | 5.17 MiB | (bundle; app dir) |
| `Claude Config Manager_0.1.0_aarch64.dmg` | 2.59 MiB | `bfd760ea81a3a7a3f895c6e53c01d3d89b586155270bb44f7cbd7ddf415fe9d2` |

- Command: `npm run tauri build` (Tauri CLI 2.12.4, rustc 1.99.1, node 26.10.0, aarch64-apple-darwin)
- Bundle id: `io.github.shahid8142.claude-config-manager`, targets `all` (`.app` + `.dmg`)
- Binary: `Mach-O 64-bit executable arm64` — **Apple Silicon only** (no universal build; Intel Macs need a
  separate target and a Rosetta/x86_64 build).

## Signing state (measured, not assumed)
```
CodeDirectory flags=0x20002(adhoc,linker-signed)
Signature=adhoc
TeamIdentifier=not set
Info.plist=not bound
Sealed Resources=none
spctl -a -vv → "code has no resources but signature indicates they must be present"
```
This is **not** merely "unsigned but working": the ad-hoc link-time signature makes Gatekeeper's
assessment fail with a *resource-envelope* error, which on current macOS typically surfaces to the user as
**"…is damaged and can't be opened"** rather than the friendlier "unidentified developer" prompt. The
documented workarounds (`xattr -d com.apple.quarantine`, right-click → Open) are **not reliable for this
signature state** and must not be presented as a supported install path.

## Consequences (honest)
- **Do not distribute this DMG.** It is a local test artifact proving the release pipeline builds and
  bundles, nothing more.
- First-launch experience on a clean Mac is expected to fail. That is a credential problem, not a code
  problem — no amount of app code fixes it.
- Because the app cannot be launched normally here, **no GUI smoke test was performed** (T65 owns that and
  currently has no evidence).

## What a real release requires (owner actions)
1. Apple Developer Program membership → **Developer ID Application** certificate.
2. Hardened runtime + the minimum entitlements (`com.apple.security.network.client` for provider calls;
   Keychain access via the `security` CLI needs no extra entitlement, but the app must be signed with a
   stable identifier so Keychain ACLs stay stable across updates).
3. `codesign --options runtime --timestamp` over the `.app` (and every nested binary).
4. `xcrun notarytool submit --wait` + `xcrun stapler staple` on the DMG.
5. Verification: `spctl -a -vv` must print `accepted` and `source=Notarized Developer ID`.
6. A universal build (`aarch64-apple-darwin` + `x86_64-apple-darwin`) if Intel Macs are in scope.

## Reproducibility notes
- The build is deterministic only in the sense that the same toolchain versions produce the same bundle;
  timestamps inside the bundle and the DMG differ per run, so the SHA-256 above identifies *this* artifact
  only.
- `src-tauri/target/` (4.3 GB) is gitignored; artifacts are not committed. The build log is kept out of git
  (`*.log`) and contains no secrets (no signing identity, no credentials).

## Acceptance criteria
- [x] Package built successfully from a clean `npm run tauri build`
- [x] Signature/notarization state measured and reported exactly
- [x] Limitation documented as a release blocker requiring owner credentials
- [ ] Signed + notarized package — **BLOCKED** (no Apple credentials; owner decision, rule 15)
- [ ] Clean-install verification — **NOT TESTED** (T65)

## Known limitations
- arm64-only; no universal binary.
- No app icon review, no macOS version floor (`minimumSystemVersion`) declared beyond Tauri defaults.
- No auto-update channel; every install is manual.
