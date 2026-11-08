import type { EffectiveStatus } from "./ipc";

/**
 * One sentence for the header pill: what Claude Code will actually use right
 * now. Kept out of the component file so fast refresh stays component-only.
 */
export function connectionLabel(status: EffectiveStatus | null): string {
  if (status === null) return "reading…";
  if (!status.settingsFound) return "not configured";
  if (status.apiKeyHelper !== null) return "connected";
  if (status.apiKeyPresent || status.authTokenPresent) return "connected (plaintext)";
  return "no credential";
}
