/**
 * Session-scoped UI state that survives relaunch (D1 of DESIGN-DECISIONS.md).
 * macOS convention: restore the last-displayed panel and working context.
 * Everything here is view state only — never secrets, never settings values.
 */
const PREFIX = "ccm.session.";

function safeStorage(): Storage | null {
  try {
    return globalThis.localStorage ?? null;
  } catch {
    return null;
  }
}

export function loadSessionString(key: string): string | null {
  try {
    return safeStorage()?.getItem(PREFIX + key) ?? null;
  } catch {
    return null;
  }
}

export function saveSessionString(key: string, value: string): void {
  try {
    safeStorage()?.setItem(PREFIX + key, value);
  } catch {
    /* private mode / quota — session-only fallback */
  }
}

export function clearSessionString(key: string): void {
  try {
    safeStorage()?.removeItem(PREFIX + key);
  } catch {
    /* ignore */
  }
}
