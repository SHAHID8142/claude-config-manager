/** Theme controller: system | light | dark, persisted in localStorage. */
export type ThemeMode = "system" | "light" | "dark";
export type ResolvedTheme = "light" | "dark";

const STORAGE_KEY = "ccm.theme";

export function resolveTheme(mode: ThemeMode, systemPrefersDark: boolean): ResolvedTheme {
  if (mode === "system") return systemPrefersDark ? "dark" : "light";
  return mode;
}

export function isThemeMode(value: unknown): value is ThemeMode {
  return value === "system" || value === "light" || value === "dark";
}

export function loadTheme(storage: Pick<Storage, "getItem"> | null = safeStorage()): ThemeMode {
  try {
    const raw = storage?.getItem(STORAGE_KEY);
    return isThemeMode(raw) ? raw : "system";
  } catch {
    return "system";
  }
}

export function saveTheme(
  mode: ThemeMode,
  storage: Pick<Storage, "setItem"> | null = safeStorage(),
): void {
  try {
    storage?.setItem(STORAGE_KEY, mode);
  } catch {
    /* private mode / quota — the choice is still applied for this session */
  }
}

export function applyTheme(
  mode: ThemeMode,
  root: HTMLElement = document.documentElement,
  systemPrefersDark = globalThis.matchMedia?.("(prefers-color-scheme: dark)").matches ?? false,
): ResolvedTheme {
  const resolved = resolveTheme(mode, systemPrefersDark);
  root.classList.toggle("dark", resolved === "dark");
  root.dataset.theme = resolved;
  root.style.colorScheme = resolved;
  return resolved;
}

function safeStorage(): Storage | null {
  try {
    return globalThis.localStorage ?? null;
  } catch {
    return null;
  }
}
