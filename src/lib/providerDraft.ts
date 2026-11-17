import { loadSessionString, saveSessionString } from "./session";
import { validateModelId } from "./modelId";
import { EMPTY_ROLES, type RoleState } from "./roles";
import { classifyEndpoint } from "./endpoint";
export interface ProviderDraft {
  model: string | null;
  roles: RoleState;
  context: string;
  resetContext: boolean;
  resetRoles: string[];
  customName: string;
  customEndpoint: string;
}
const empty = (): ProviderDraft => ({
  model: null,
  roles: { ...EMPTY_ROLES },
  context: "",
  resetContext: false,
  resetRoles: [],
  customName: "",
  customEndpoint: "",
});
function clean(value: unknown): ProviderDraft {
  const out = empty();
  if (!value || typeof value !== "object") return out;
  const v = value as Record<string, unknown>;
  const model = (raw: unknown) =>
    typeof raw === "string" && validateModelId(raw).ok ? raw.trim() : null;
  out.model = model(v.model);
  const roles = v.roles as Record<string, unknown> | undefined;
  for (const key of ["haiku", "sonnet", "opus"] as const)
    out.roles[key] = model(roles?.[key]) ?? "";
  if (typeof v.context === "string" && /^\d{1,8}$/.test(v.context)) out.context = v.context;
  out.resetContext = v.resetContext === true;
  if (Array.isArray(v.resetRoles))
    out.resetRoles = v.resetRoles.filter(
      (k): k is string => typeof k === "string" && ["haiku", "sonnet", "opus"].includes(k),
    );
  if (typeof v.customEndpoint === "string" && v.customEndpoint.length < 2048) {
    try {
      const u = new URL(v.customEndpoint);
      if (
        !u.username &&
        !u.password &&
        !u.search &&
        !u.hash &&
        ["remote_ok", "local"].includes(classifyEndpoint(v.customEndpoint).check) &&
        !/(?:sk-ant-|sk-or-|nvapi-|ghp_)/i.test(u.pathname)
      )
        out.customEndpoint = v.customEndpoint;
    } catch {
      /* incomplete URL remains session-only */
    }
  }
  if (
    out.customEndpoint &&
    typeof v.customName === "string" &&
    v.customName.length <= 80 &&
    !Array.from(v.customName).some((c) => c.charCodeAt(0) < 32) &&
    !/(?:sk-ant-|sk-or-|nvapi-|ghp_)/i.test(v.customName)
  )
    out.customName = v.customName;
  return out;
}
function drafts(): Record<string, ProviderDraft> {
  try {
    const raw = loadSessionString("provider-drafts-v1");
    if (!raw || raw.length > 65536) return {};
    const parsed: unknown = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) return {};
    return Object.fromEntries(
      Object.entries(parsed)
        .slice(-32)
        .filter(([key]) => /^[a-z0-9_-]{1,80}$/.test(key))
        .map(([key, value]) => [key, clean(value)]),
    );
  } catch {
    return {};
  }
}
export function loadProviderDraft(id: string): ProviderDraft {
  return drafts()[id] ?? empty();
}
export function saveProviderDraft(id: string, draft: ProviderDraft, customId?: string) {
  if (!/^[a-z0-9_-]{1,80}$/.test(id) || id === "__none__") return;
  const all = drafts();
  const safe = clean(draft);
  all[id] = safe;
  if (customId && safe.customEndpoint && safe.model) all[customId] = safe;
  saveSessionString(
    "provider-drafts-v1",
    JSON.stringify(Object.fromEntries(Object.entries(all).slice(-32))),
  );
}
export function customProviderDrafts(): Array<{ id: string; draft: ProviderDraft }> {
  return Object.entries(drafts())
    .filter(([id, draft]) => id.startsWith("custom-") && draft.customEndpoint)
    .map(([id, draft]) => ({ id, draft }));
}
