import { validateModelId } from "./modelId";

/** Model-role override state (T45). Blank string = "no override". */
export interface RoleState {
  haiku: string;
  sonnet: string;
  opus: string;
}

export const EMPTY_ROLES: RoleState = { haiku: "", sonnet: "", opus: "" };

export type RoleKey = keyof RoleState;

export interface RoleField {
  key: RoleKey;
  label: string;
  env: string;
}

export const ROLE_FIELDS: readonly RoleField[] = [
  { key: "haiku", label: "Haiku role", env: "ANTHROPIC_DEFAULT_HAIKU_MODEL" },
  { key: "sonnet", label: "Sonnet role", env: "ANTHROPIC_DEFAULT_SONNET_MODEL" },
  { key: "opus", label: "Opus role", env: "ANTHROPIC_DEFAULT_OPUS_MODEL" },
] as const;

export type RoleCheck =
  | { key: RoleKey; kind: "none" }
  | { key: RoleKey; kind: "valid"; value: string }
  | { key: RoleKey; kind: "invalid"; message: string };

/**
 * Validate each role override. Blank means "no override": the app writes
 * nothing and Claude Code's own role default applies (PRD F05 — never invent
 * a mapping).
 */
export function roleChecks(roles: RoleState): RoleCheck[] {
  return ROLE_FIELDS.map(({ key }) => {
    const raw = roles[key];
    if (raw.trim() === "") return { key, kind: "none" } as RoleCheck;
    const check = validateModelId(raw);
    return check.ok
      ? ({ key, kind: "valid", value: check.value } as RoleCheck)
      : ({ key, kind: "invalid", message: check.message } as RoleCheck);
  });
}

export function hasInvalidRole(roles: RoleState): boolean {
  return roleChecks(roles).some((c) => c.kind === "invalid");
}

/** Only validated overrides are ever sent; blank/absent stays `undefined`. */
export function rolesToSend(roles: RoleState): Partial<Record<RoleKey, string>> {
  const out: Partial<Record<RoleKey, string>> = {};
  for (const check of roleChecks(roles)) {
    if (check.kind === "valid") out[check.key] = check.value;
  }
  return out;
}
