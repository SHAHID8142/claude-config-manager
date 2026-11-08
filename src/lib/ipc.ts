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
