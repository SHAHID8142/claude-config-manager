import type { EffectiveStatus, StatusWarning } from "./ipc";

/** Human phrase for how Claude Code is reaching a provider. */
export function modeLabel(status: EffectiveStatus | null): string {
  if (status === null) return "unknown";
  if (!status.settingsFound) return "not configured";
  if (status.apiKeyHelper !== null) return "direct (credential helper)";
  if (status.apiKeyPresent || status.authTokenPresent) return "direct (plaintext env)";
  return "not configured";
}

/**
 * One sentence per warning variant, phrased as what the owner would do.
 *
 * Serde emits unit variants as bare strings and struct variants as one-key
 * objects, so `typeof` is checked before any `in` (the black-screen bug).
 */
export function warningText(w: StatusWarning): string {
  if (typeof w === "string") {
    if (w === "helperShadowedByEnv") {
      return "apiKeyHelper is shadowed by a plaintext env credential — remove the env key to use the helper.";
    }
    return "Unknown warning";
  }
  if ("helperShadowedByEnv" in w) {
    return "apiKeyHelper is shadowed by a plaintext env credential — remove the env key to use the helper.";
  }
  if ("plaintextKeyInEnv" in w) {
    return `${w.plaintextKeyInEnv.which} is stored in plaintext inside settings.json.`;
  }
  if ("modelConflict" in w) {
    return `Model conflict: settings say "${w.modelConflict.settingModel}" but ANTHROPIC_MODEL says "${w.modelConflict.envModel}" (env wins).`;
  }
  return "Unknown warning";
}

/** True when a status needs the owner to look at something. */
export function needsAttention(status: EffectiveStatus | null): boolean {
  return status !== null && status.settingsFound && status.warnings.length > 0;
}

/** "1,000,000" or "provider default" — never a bare number without units. */
export function contextWindowLabel(raw: string | null | undefined): string {
  if (raw === null || raw === undefined || raw.trim() === "") return "provider default";
  return `${Number(raw).toLocaleString()} tokens`;
}
