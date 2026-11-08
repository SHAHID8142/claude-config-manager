// Mock IPC transport for UI development and contract tests (T11).
//
// A mock implements `IpcLike` by dispatching on command name, returning
// fixtures or simulated failures. Contract tests pin the shapes here before
// real handlers exist, so later Rust wiring cannot silently drift.

import type { CommandError, IpcLike } from "./ipc";

export type MockHandler = (args: Record<string, unknown> | undefined) => Promise<unknown>;

export interface MockIpcOptions {
  /** Return a rejection for a command name (must be a CommandError shape). */
  failures?: Record<string, CommandError>;
  /** Override default fixture handlers. */
  handlers?: Record<string, MockHandler>;
}

export interface MockIpc extends IpcLike {
  /** Commands invoked so far, in order — used by tests for interaction checks. */
  calls: Array<{ cmd: string; args?: Record<string, unknown> }>;
  /** Install/replace a handler at runtime. */
  on(cmd: string, handler: MockHandler): void;
}

export function createMockIpc(options: MockIpcOptions = {}): MockIpc {
  const handlers: Record<string, MockHandler> = { ...(options.handlers ?? {}) };
  const failures: Record<string, CommandError> = { ...(options.failures ?? {}) };
  const calls: MockIpc["calls"] = [];

  const ipc: MockIpc = {
    calls,
    on(cmd, handler) {
      handlers[cmd] = handler;
    },
    invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
      calls.push({ cmd, args });
      const failure = failures[cmd];
      if (failure) {
        return Promise.reject(failure) as Promise<T>;
      }
      const handler = handlers[cmd];
      if (!handler) {
        return Promise.reject({
          kind: "not_available",
          message: `no mock handler for command '${cmd}'`,
        } satisfies CommandError) as Promise<T>;
      }
      return handler(args).then((value) => value as T);
    },
  };
  return ipc;
}
