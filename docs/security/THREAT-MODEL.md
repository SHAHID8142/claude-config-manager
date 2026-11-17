# Current threat model — 1.0.6

Reviewed 2026-11-17. Supersedes the v1 Keychain and external-bridge design claims.
The supported distribution target is Apple Silicon macOS. The threat boundary
is a single user's desktop; another process running as that user can read private
plaintext data. Filesystem permissions are not encryption or same-user isolation.

| Risk | Mitigation and residual limit |
| --- | --- |
| Credential exposure | Keys use 0600 files, private unique temporaries, bounded no-follow reads and strict identifiers. No raw keys in settings/helper scripts. Same-user processes remain able to read keys. |
| Private recovery copies | Journals/recovery snapshots are 0600 and contain base64 before-images, including keys. Never share these files. Base64 is not encryption. Existing settings backups may also contain old plaintext credentials. |
| Invalid settings/data loss | Invalid input is refused. Confirmed recovery preserves invalid original bytes. Selected backups are validated and previewed. No silent replacement with an empty object. |
| Interrupted multi-file operations | Before-image journal + shared lock; explicit restore/keep after a crash. Rollback failures are surfaced and retain the marker. This is not crash-atomic multi-file storage; disk failure can prevent recovery. |
| Concurrent writers | Shared cross-process lock serializes cooperating writers; settings conflict checks preserve later external edits on failed Apply. A final check/rename race with non-cooperating editors remains possible. |
| Symlink/path traversal | Owned parents and regular-file checks, no-follow bounded reads, strict backup basenames and journal path allowlists. Legitimate home/system aliases are handled deliberately. No filesystem-level sandbox against a malicious same-user actor. |
| Shell injection | Provider/account identifiers are validated. Fixed generated scripts quote HOME paths; apiKeyHelper commands encode exactly one executable path, including spaces/quotes/metacharacters. The app does not accept arbitrary helper commands through Apply. |
| Wrong-provider credential forwarding | Vault ownership/enabled checks and endpoint-scoped custom IDs. Provider change clears typed key input. HTTPS and parsed-host loopback rules; redirects do not forward credentials. User-selected gateways still receive the keys/prompts sent to them. |
| Local proxy abuse | Authenticated 127.0.0.1 listener, limited request paths, bounded headers/body/clients/attempts. Authentication token is private. A same-user actor with file access can obtain it. |
| Malformed provider responses | Bounded catalog, response, SSE frame and tool-argument parsing. Unsupported NVIDIA content/features fail explicitly. No replay after response streaming begins. A timed-out upstream may still process/charge a request. |
| Quota/auth ambiguity | HTTP outcomes are distinguished from confirmed key health. Keys may share account quotas. No promise that changing keys creates quota or bypasses limits. |
| Misleading status | Saved global scope, drafts, key checks and inference evidence are separated. Old direct NVIDIA configuration warns that activation is needed. Project/managed/terminal settings and CLI helper cache can override the apparent state. |
| Activity/privacy | Bounded history contains only timestamps, provider IDs and fixed outcomes; no endpoints, prompts/responses, key values or key labels. History failures are reported; history is not a billing or full request ledger. |
| Draft storage | Whitelisted validated non-secret fields only, bounded local storage; key inputs omitted. WebKit/cache cleanup removes drafts. Invalid/incomplete fields are session-only. |
| Helper lifecycle | PID + responding health identity checked before signals. Detached helper survives GUI exit; login start is opt-in. Real reboot/clean-Mac acceptance remains unverified. |
| Dependency/distribution compromise | Committed npm/Cargo locks, npm/OSV audit, gitleaks source/history review and artifact hashes. Dormant external bridge execution is disabled; its separate advisory records remain documented. No supply-chain audit can prove absence of malicious dependencies. |
| macOS trust | Ad-hoc signature integrity checked. No Developer ID/notarization; Gatekeeper rejects this unsigned prerelease. No security protections are disabled automatically. |

Review evidence: [audit and register](../audits/2026-11-17-project-review.md),
[release notes](../releases/1.0.6.md), and focused fixture-home security, recovery,
property, protocol and real CLI tool tests. Do not upload private files to reproduce
bugs; use synthetic fixture data. A test pass does not establish universal safety.
