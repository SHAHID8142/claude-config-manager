import type { EffectiveStatus } from "./ipc";

/**
 * V3-24 — what Claude Code will actually do right now.
 *
 * A key file on disk is *configured*, not connected: V2 called that state
 * "connected" without one network request ever succeeding. Every state here is
 * something the app can prove or explicitly cannot.
 */
export type ConnectionState =
  | "checking"
  | "not-configured"
  | "configured"
  | "verified"
  | "unavailable"
  | "proxy-stopped"
  | "needs-attention";

export interface ConnectionContext {
  /** The detatched helper answered /healthz. */
  proxyRunning?: boolean;
  /** Result of an explicit Test Connection in this session. */
  lastTest?: { ok: boolean } | null;
}

export interface ConnectionDescription {
  state: ConnectionState;
  /** Short pill / summary text. */
  label: string;
  /** One sentence for the summary strip. */
  detail: string;
}

export function isLoopback(url: string | null | undefined): boolean {
  if (url === null || url === undefined) return false;
  return url.startsWith("http://127.0.0.1") || url.startsWith("http://localhost");
}

export function describeConnection(
  status: EffectiveStatus | null,
  ctx: ConnectionContext = {},
): ConnectionDescription {
  if (status === null) {
    return { state: "checking", label: "checking", detail: "Reading the Claude Code settings…" };
  }
  if (!status.settingsFound) {
    return {
      state: "not-configured",
      label: "not configured",
      detail: "Claude Code has no configuration yet.",
    };
  }
  if (isLoopback(status.envBaseUrl)) {
    if (ctx.proxyRunning === false) {
      return {
        state: "proxy-stopped",
        label: "proxy stopped",
        detail: "Claude Code points at the proxy, but the proxy helper is not running.",
      };
    }
    return {
      state: "configured",
      label: "routed",
      detail: "Claude Code sends requests to the local failover proxy.",
    };
  }
  if (ctx.lastTest?.ok === true) {
    return {
      state: "verified",
      label: "verified",
      detail: "A request to the provider was accepted in this session.",
    };
  }
  if (ctx.lastTest?.ok === false) {
    return {
      state: "unavailable",
      label: "unavailable",
      detail: "The last connection test failed; Claude Code will not reach this provider.",
    };
  }
  if (status.apiKeyPresent || status.authTokenPresent) {
    return {
      state: "needs-attention",
      label: "needs attention",
      detail: "A key is stored in plain text in the settings file. Move it into the app's store.",
    };
  }
  if (status.apiKeyHelper !== null) {
    return {
      state: "configured",
      label: "configured",
      detail: "Claude Code has a credential helper. It has not been verified in this session.",
    };
  }
  if (status.envBaseUrl !== null || status.model !== null) {
    return {
      state: "needs-attention",
      label: "needs attention",
      detail: "Claude Code is configured but has no credential.",
    };
  }
  return {
    state: "not-configured",
    label: "not configured",
    detail: "Claude Code has no configuration yet.",
  };
}

/** Header pill text: the state, spelled the same way everywhere. */
export function connectionLabel(
  status: EffectiveStatus | null,
  ctx: ConnectionContext = {},
): string {
  return describeConnection(status, ctx).label;
}
