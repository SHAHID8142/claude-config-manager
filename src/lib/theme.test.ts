import { describe, expect, it, vi } from "vitest";
import { applyTheme, isThemeMode, loadTheme, resolveTheme, saveTheme } from "./theme";

describe("theme", () => {
  it("resolves system against the OS preference", () => {
    expect(resolveTheme("system", true)).toBe("dark");
    expect(resolveTheme("system", false)).toBe("light");
    expect(resolveTheme("light", true)).toBe("light");
    expect(resolveTheme("dark", false)).toBe("dark");
  });

  it("only accepts known modes from storage", () => {
    expect(isThemeMode("dark")).toBe(true);
    expect(isThemeMode("DARK")).toBe(false);
    expect(isThemeMode(null)).toBe(false);
  });

  it("defaults to system when storage is empty or hostile", () => {
    expect(loadTheme({ getItem: () => null })).toBe("system");
    expect(loadTheme({ getItem: () => "neon" })).toBe("system");
    expect(
      loadTheme({
        getItem: () => {
          throw new Error("blocked");
        },
      }),
    ).toBe("system");
  });

  it("persists without throwing when storage is unavailable", () => {
    const setItem = vi.fn();
    saveTheme("dark", { setItem });
    expect(setItem).toHaveBeenCalledWith("ccm.theme", "dark");
    expect(() =>
      saveTheme("dark", {
        setItem: () => {
          throw new Error("quota");
        },
      }),
    ).not.toThrow();
  });

  it("toggles the class and color-scheme on the root element", () => {
    const root = document.createElement("div");
    expect(applyTheme("dark", root, false)).toBe("dark");
    expect(root.classList.contains("dark")).toBe(true);
    expect(root.style.colorScheme).toBe("dark");
    expect(applyTheme("light", root, true)).toBe("light");
    expect(root.classList.contains("dark")).toBe(false);
    expect(root.style.colorScheme).toBe("light");
  });
});
