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
  });
}

// ---------------------------------------------------------------------------
// `read_status` (T34) — effective-config read-back with override warnings.
// ---------------------------------------------------------------------------

export type StatusWarning =
  | { helperShadowedByEnv: null }
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
}

export async function checkEnvironment(ipc: IpcLike = defaultIpc): Promise<EnvironmentReport> {
  return ipc.invoke<EnvironmentReport>("check_environment");
}
