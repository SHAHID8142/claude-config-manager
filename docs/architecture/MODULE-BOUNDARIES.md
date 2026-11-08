# Module Boundaries (T06)

**Date:** 2026-11-08 · **Binding for all implementation tasks** · Matches `02-ARCHITECTURE-SECURITY.md`: UI → typed Tauri commands → Rust application service → five domain modules. A small `CLIAdapter` interface; **only `ClaudeCodeAdapter` implemented** — no speculative plugin framework.

## Data flow (one direction only)

```
React UI (TS)
  └─ typed invoke() per contract in T11 ──►  Tauri commands (thin: validate, delegate, map errors)
        └─►  application service (AppService: orchestration only, no I/O policy of its own)
              ├─ ClaudeCodeAdapter   (implements CLIAdapter)
              ├─ ProviderCatalog     (versioned templates)
              ├─ SecretStore         (Keychain-backed)
              ├─ ConfigWriter        (backup/merge/atomic write)
              └─ BridgeManager       (download/install/run/stop/uninstall)
```

Rules: UI never touches fs/network/Keychain directly; Tauri command handlers never embed domain logic; domain modules never import Tauri types (testable with plain Rust tests).

## CLIAdapter (trait) — the only CLI abstraction

```rust
pub trait CLIAdapter: Send + Sync {
    /// Locate the global config file for this CLI (honors CLAUDE_CONFIG_DIR).
    fn locate_global_config(&self, home: &Path) -> Result<PathBuf, ConfigError>;
    /// Read global config as raw JSON (no mutation, no schema assumptions).
    fn read_global_config(&self, path: &Path) -> Result<serde_json::Value, ConfigError>;
    /// Targeted merge: returns new document; caller owns backup/atomic write via ConfigWriter.
    fn apply_settings(&self, current: &serde_json::Value, desired: &DesiredSettings) -> Result<serde_json::Value, ConfigError>;
    /// Keys this adapter claims as its own; everything else is "unrelated, preserve".
    fn supported_keys(&self) -> &'static [SupportedKey];
    /// Effective-config read-back with override warnings (T34).
    fn effective_status(&self, path: &Path, desired: &DesiredSettings) -> Result<EffectiveStatus, ConfigError>;
}
pub struct ClaudeCodeAdapter; // sole implementation (v1). Codex/Gemini/OpenCode: future, not scaffolded.
```

Error taxonomy `ConfigError`: `Missing`, `MalformedJson{source}`, `PermissionDenied`, `SymlinkRefused`, `ConcurrentModification{expected_mtime}`, `UnsupportedKey`, `ValidationFailed{field, reason}`, `Io{source}` — all user-mappable, all redaction-safe.

## ProviderCatalog

```rust
pub struct ProviderTemplate { // schema v1, versioned field (T16)
    id: ProviderId, schema_version: u32,
    name: String,
    auth: AuthStyle,                    // ApiKey | BearerToken (maps to env var choice, T02 matrix)
    anthropic_base_url: Option<Url>,    // direct mode endpoint (None = native Anthropic)
    protocol: Protocol,                 // AnthropicNative | AnthropicCompat | OpenAiOnly
    bridge: BridgeRequirement,          // NotRequired | RequiredForOpenAiOnly
    discovery: DiscoverySpec,           // None | Endpoint{url, auth, selector}
    capability_status: CapabilityStatus,// Verified{source_url} | Unknown — never fabricated
}
```
Owns: preset loading, template validation, capability labels (T25). Forbidden: network calls (discovery execution lives in ModelDiscovery service, T37), credential access.

## SecretStore (trait)

```rust
pub trait SecretStore: Send + Sync {
    fn set_secret(&self, service: &str, account: &str, secret: &str) -> Result<(), SecretError>;
    fn get_secret(&self, service: &str, account: &str) -> Result<Option<String>, SecretError>;
    fn delete_secret(&self, service: &str, account: &str) -> Result<(), SecretError>;
}
```
Impls: `KeychainStore` (macOS, T22), `MockSecretStore` (tests). Contract: no secret ever returned into log/error paths; errors carry no secret fragments (T23 tests). Service name: `com.claude-config-manager.<provider>`.

## ConfigWriter

Sole owner of `~/.claude/settings.json` writes. Pipeline (each step a testable unit):
`locate → read (refuse symlink) → validate (strict JSON) → merge (adapter.apply_settings) → backup (0600, app dir) → conflict check (mtime) → temp file in same dir → fsync → atomic rename → read-back verify → rollback on mismatch`.
Forbidden: writing outside the located config path; writing `~/.claude.json`; emitting non-strict JSON; touching project-level settings (T02 matrix).

## BridgeManager

Owns bridge lifecycle exclusively: consent state, pinned download + checksum verify (T49), app-owned install dir `~/Library/Application Support/ClaudeConfigManager/bridge/` (T50), loopback-only config generation (T51), start/stop/status with failure recovery (T52), standalone-run lifecycle for GUI closure (T53), uninstall + direct-mode switch (T58).
Forbidden: executing unverified binaries; binding non-loopback; spawning shells with user input; managing anything outside its install dir.

## Cross-cutting policies (enforced, not advisory)

1. **No shell interpolation with user input** anywhere (threat T6).
2. **No capability fabrication**: capability data only from verified sources; UI labels verified/unknown/bridge-required (T25).
3. **Never auto-select** a model (F04) — enforced in Rust layer, not just UI.
4. **No secrets across boundaries**: IPC responses, errors, logs, backups pass through redaction (T23).
5. **Version-sensitive facts** (settings keys, provider protocols) cite `docs/research/*` matrices; re-verify before implementing each consuming task (rule 9).
