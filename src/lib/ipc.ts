// Typed UI↔Rust command contracts (T11).
//
// Rules:
// - Every command is a typed function with an explicit result type; raw
//   `invoke(cmd, args)` calls outside this module are forbidden.
// - Rust commands return `Result<T, CommandError>`; failures surface as
//   `CommandError` objects (see `isCommandError`).
// - New commands are added here in the same task that adds their Rust handler.

import { invoke } from "@tauri-apps/api/core";

/** Minimal transport seam so UI code and tests can swap real/mock IPC. */
export interface IpcLike {
  invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T>;
}

/** Real transport backed by the Tauri runtime. */
export const defaultIpc: IpcLike = {
  invoke: <T>(cmd: string, args?: Record<string, unknown>) => invoke<T>(cmd, args),
};

/** Mirrors Rust `CommandError` (serde tag/content). */
export interface CommandError {
  kind: "invalid_input" | "io" | "not_available";
  message: string;
}

export function isCommandError(value: unknown): value is CommandError {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return (
    (v.kind === "invalid_input" || v.kind === "io" || v.kind === "not_available") &&
    typeof v.message === "string"
  );
}

// ---------------------------------------------------------------------------
// Existing Rust commands (src-tauri/src/lib.rs). Keep in sync.
// ---------------------------------------------------------------------------

/** `greet` — scaffold command; negative case: names >256 chars reject. */
export async function greet(name: string, ipc: IpcLike = defaultIpc): Promise<string> {
  if (name.length > 256) {
    // Client-side mirror of the Rust guard: fail fast with the same shape.
    return Promise.reject({
      kind: "invalid_input",
      message: "name exceeds 256 characters",
    } satisfies CommandError);
  }
  return ipc.invoke<string>("greet", { name });
}

// ---------------------------------------------------------------------------
// `list_providers` (T18) — bundled provider templates for the selector.
// ---------------------------------------------------------------------------

export interface ProviderSummary {
  id: string;
  name: string;
  protocol: "anthropic_native" | "anthropic_compat" | "open_ai_only";
  bridge: "not_required" | "required_for_open_ai_only";
  capabilityStatus: { status: "verified"; source_url: string } | { status: "unknown" };
  docsUrl: string | null;
  /** Direct base URL for compat presets; null for native/OpenAI-only. */
  directBaseUrl: string | null;
}

export async function listProviders(ipc: IpcLike = defaultIpc): Promise<ProviderSummary[]> {
  return ipc.invoke<ProviderSummary[]>("list_providers");
}

// ---------------------------------------------------------------------------
// `save_and_apply` (T33) — full safe-apply pipeline for one direct provider.
// ---------------------------------------------------------------------------

export interface ApplyOutcome {
  settingsPath: string;
  backupFileName: string;
  helperPath: string;
  appliedKeys: string[];
}

export interface SaveApplyArgs {
  providerId: string;
  model?: string;
  baseUrl?: string;
  apiKey?: string;
  /**
   * Optional model-role overrides (T45). A missing key means "no override":
   * the managed env var is left untouched, never guessed.
   */
  roleModels?: { haiku?: string; sonnet?: string; opus?: string };
  /**
   * Declared context window for a model Claude Code does not recognize
   * (V2-12b). Without it Claude Code assumes 200k and compacts there even when
   * the model really supports 1M. Omit to leave the managed key untouched.
   */
  maxContextTokens?: number;
  /**
   * Which stored credential the generated helper must read (V2-21). Absent
   * keeps the pre-V2 provider-level file.
   */
  credentialId?: string;
  /** Remove `CLAUDE_CODE_MAX_CONTEXT_TOKENS` instead of writing a value. */
  resetContextWindow?: boolean;
}

export async function saveAndApply(
  args: SaveApplyArgs,
  ipc: IpcLike = defaultIpc,
): Promise<ApplyOutcome> {
  return ipc.invoke<ApplyOutcome>("save_and_apply", {
    providerId: args.providerId,
    model: args.model ?? null,
    baseUrl: args.baseUrl ?? null,
    apiKey: args.apiKey ?? null,
    roleHaiku: args.roleModels?.haiku ?? null,
    roleSonnet: args.roleModels?.sonnet ?? null,
    roleOpus: args.roleModels?.opus ?? null,
    credentialId: args.credentialId ?? null,
    maxContextTokens: args.maxContextTokens ?? null,
    resetContextWindow: args.resetContextWindow ?? false,
  });
}

// ---------------------------------------------------------------------------
// `read_status` (T34) — effective-config read-back with override warnings.
// ---------------------------------------------------------------------------

/**
 * Mirrors the Rust `StatusWarning` enum as serde actually emits it.
 *
 * Serde serialises a **unit** variant as a bare string, and a struct variant as a
 * single-key object. Getting this wrong is not cosmetic: `"x" in "string"` throws
 * a TypeError during render, which unmounts the React tree and leaves a blank
 * window (the black-screen bug).
 */
export type StatusWarning =
  | "helperShadowedByEnv"
  | { plaintextKeyInEnv: { which: string } }
  | { modelConflict: { settingModel: string; envModel: string } };

export interface EffectiveStatus {
  settingsFound: boolean;
  model: string | null;
  envModel: string | null;
  envBaseUrl: string | null;
  authTokenPresent: boolean;
  apiKeyPresent: boolean;
  apiKeyHelper: string | null;
  /** T45: configured role overrides; null = Claude Code default applies. */
  envDefaultHaiku: string | null;
  envDefaultSonnet: string | null;
  envDefaultOpus: string | null;
  /** V2-12b: declared window for unrecognized ids; null = 200k assumption. */
  envMaxContextTokens: string | null;
  warnings: StatusWarning[];
}

export async function readStatus(ipc: IpcLike = defaultIpc): Promise<EffectiveStatus> {
  return ipc.invoke<EffectiveStatus>("read_status");
}

// ---------------------------------------------------------------------------
// `restore_last_backup` (T35) — newest backup with pre-restore safety copy.
// ---------------------------------------------------------------------------

export interface RestoreOutcome {
  restoredFrom: string;
  preRestoreBackup: string;
}

export async function restoreLastBackup(ipc: IpcLike = defaultIpc): Promise<RestoreOutcome> {
  return ipc.invoke<RestoreOutcome>("restore_last_backup");
}

// ---------------------------------------------------------------------------
// `check_environment` (T36) — CLI presence/version + settings diagnostics.
// ---------------------------------------------------------------------------

export interface EnvironmentReport {
  cliFound: boolean;
  cliVersion: string | null;
  cliVersionWarning: string | null;
  /** ok | first_run | malformed_json | not_a_file | symlink | unreadable */
  settingsState: string;
  settingsDetail: string | null;
  configDirWritable: boolean;
  /** ANTHROPIC_* exported by the login shell — names and lengths only. */
  shellEnv: Array<{ name: string; length: number }>;
}

export async function checkEnvironment(ipc: IpcLike = defaultIpc): Promise<EnvironmentReport> {
  return ipc.invoke<EnvironmentReport>("check_environment");
}

// ---------------------------------------------------------------------------
// `discover_models` (T38) — catalog fetch for one verified provider.
// ---------------------------------------------------------------------------

export interface ModelEntry {
  id: string;
  displayName: string | null;
  /** Always "unknown" at discovery time — never invented (F12). */
  capabilityStatus: "unknown";
  /** Provenance: the verified catalog URL this record was read from (T42). */
  source: string;
}

export interface ConnectionTestResult {
  providerId: string;
  ok: boolean;
  status: number | null;
  message: string;
  endpoint: string;
  latencyMs: number;
}

export async function testConnection(
  providerId: string,
  credential?: string,
  ipc: IpcLike = defaultIpc,
): Promise<ConnectionTestResult> {
  return ipc.invoke<ConnectionTestResult>("test_connection", {
    providerId,
    credential: credential ?? null,
  });
}

// ---------------------------------------------------------------------------
// Credentials (V2-21) — masked metadata only; the key itself never round-trips
// back to the UI.
// ---------------------------------------------------------------------------

export interface CredentialView {
  id: string;
  providerId: string;
  label: string;
  /** Last four characters, or "" when unknown (migrated legacy keys). */
  last4: string;
  priority: number;
  enabled: boolean;
  /** Pre-V2 key at secrets/<provider>.secret — labelled, never deletable here. */
  legacy: boolean;
}

export async function listCredentials(
  providerId: string,
  ipc: IpcLike = defaultIpc,
): Promise<CredentialView[]> {
  return ipc.invoke<CredentialView[]>("list_credentials", { providerId });
}

export async function addCredential(
  providerId: string,
  label: string,
  secret: string,
  ipc: IpcLike = defaultIpc,
): Promise<CredentialView> {
  return ipc.invoke<CredentialView>("add_credential", { providerId, label, secret });
}

export async function removeCredential(
  credentialId: string,
  ipc: IpcLike = defaultIpc,
): Promise<boolean> {
  return ipc.invoke<boolean>("remove_credential", { credentialId });
}

export async function setCredentialEnabled(
  credentialId: string,
  enabled: boolean,
  ipc: IpcLike = defaultIpc,
): Promise<boolean> {
  return ipc.invoke<boolean>("set_credential_enabled", { credentialId, enabled });
}

export async function promoteCredential(
  credentialId: string,
  ipc: IpcLike = defaultIpc,
): Promise<boolean> {
  return ipc.invoke<boolean>("promote_credential", { credentialId });
}

/** V2-12a: is a credential already stored for this provider? Lets the UI keep
 *  "Save & Apply" enabled when only the model changes. */
export async function hasCredential(
  providerId: string,
  ipc: IpcLike = defaultIpc,
): Promise<boolean> {
  return ipc.invoke<boolean>("has_credential", { providerId });
}

export async function discoverModels(
  providerId: string,
  credential?: string,
  ipc: IpcLike = defaultIpc,
): Promise<ModelEntry[]> {
  return ipc.invoke<ModelEntry[]>("discover_models", {
    providerId,
    credential: credential ?? null,
  });
}

// ---------------------------------------------------------------------------
// Failover routing + loopback proxy (V2-23/26/30/31). Targets carry credential
// IDs only, never key material.
// ---------------------------------------------------------------------------

export interface RouteTarget {
  id: string;
  providerId: string;
  credentialId: string | null;
  model: string | null;
  baseUrl: string;
  enabled: boolean;
}

export interface RoutingConfig {
  version: number;
  enabled: boolean;
  targets: RouteTarget[];
}

export interface ProxyInfo {
  port: number;
  helperPath: string;
}

/** V3-26: real on-disk locations, reported by the backend. */
export interface ConfigPaths {
  home: string;
  secretsDir: string;
  helpersDir: string;
  routingFile: string;
  settingsFile: string;
  helperState: string;
}

/** V3-07/V3-09: the detached proxy helper, as the backend sees it. */
export interface HelperStatus {
  running: boolean;
  stale: boolean;
  pid: number | null;
  port: number | null;
  loginStart: boolean;
  message: string;
}

export interface ProxyStatus {
  running: boolean;
  port: number | null;
  requests: number;
  fallbacks: number;
  failures: number;
  lastTarget: string | null;
  /** owning process id, so the app can prove a state file is not stale */
  pid?: number;
  lastError: string | null;
  cooling: string[];
}

export async function getRouting(ipc: IpcLike = defaultIpc): Promise<RoutingConfig> {
  return ipc.invoke<RoutingConfig>("get_routing");
}

export async function saveRouting(
  config: RoutingConfig,
  ipc: IpcLike = defaultIpc,
): Promise<void> {
  return ipc.invoke<void>("save_routing", { config });
}

export async function proxyStart(ipc: IpcLike = defaultIpc): Promise<ProxyInfo> {
  return ipc.invoke<ProxyInfo>("proxy_start");
}

export async function proxyStop(ipc: IpcLike = defaultIpc): Promise<boolean> {
  return ipc.invoke<boolean>("proxy_stop");
}

export async function proxyStatus(ipc: IpcLike = defaultIpc): Promise<ProxyStatus | null> {
  return ipc.invoke<ProxyStatus | null>("proxy_status");
}

export async function configPaths(ipc: IpcLike = defaultIpc): Promise<ConfigPaths> {
  return ipc.invoke<ConfigPaths>("config_paths");
}

export async function proxyRestart(ipc: IpcLike = defaultIpc): Promise<ProxyInfo> {
  return ipc.invoke<ProxyInfo>("proxy_restart");
}

export async function helperStatus(ipc: IpcLike = defaultIpc): Promise<HelperStatus> {
  return ipc.invoke<HelperStatus>("helper_status");
}

/** Explicit opt-in: true installs the user LaunchAgent, false removes it. */
export async function helperLoginStart(
  enabled: boolean,
  ipc: IpcLike = defaultIpc,
): Promise<HelperStatus> {
  return ipc.invoke<HelperStatus>("helper_login_start", { enabled });
}

export async function applyRoutedMode(
  model: string | null,
  ipc: IpcLike = defaultIpc,
): Promise<ApplyOutcome> {
  return ipc.invoke<ApplyOutcome>("apply_routed_mode", { model });
}

export interface TargetHealth {
  targetId: string;
  providerId: string;
  ok: boolean;
  message: string;
}

/** V2-32: probe every enabled target at its documented key-check endpoint.
 *  Never sends a completion, so it cannot spend quota. */
export async function checkRoutingHealth(ipc: IpcLike = defaultIpc): Promise<TargetHealth[]> {
  return ipc.invoke<TargetHealth[]>("check_routing_health");
}
