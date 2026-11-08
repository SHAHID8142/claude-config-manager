import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ThemeSegmented } from "./ThemeSegmented";

function memoryStorage(initial: Record<string, string> = {}) {
  const map = new Map(Object.entries(initial));
  return {
    getItem: (k: string) => map.get(k) ?? null,
    setItem: (k: string, v: string) => void map.set(k, v),
    dump: () => Object.fromEntries(map),
  };
}

describe("ThemeSegmented", () => {
  it("reflects the stored choice and persists a new one", async () => {
    const user = userEvent.setup();
    const storage = memoryStorage({ "ccm.theme": "light" });
    render(<ThemeSegmented themeStorage={storage} />);

    expect(screen.getByRole("radio", { name: "Light" })).toHaveAttribute("aria-checked", "true");
    expect(screen.getByRole("radio", { name: "System" })).toHaveAttribute(
      "aria-checked",
      "false",
    );

    await user.click(screen.getByRole("radio", { name: "Dark" }));
    expect(screen.getByRole("radio", { name: "Dark" })).toHaveAttribute("aria-checked", "true");
    expect(storage.dump()["ccm.theme"]).toBe("dark");
    expect(document.documentElement.classList.contains("dark")).toBe(true);

    // Back to following the OS: the class is resolved from the media query.
    await user.click(screen.getByRole("radio", { name: "System" }));
    expect(storage.dump()["ccm.theme"]).toBe("system");
  });

  it("is a radio group, not a menu, so the choice is one click away", () => {
    render(<ThemeSegmented themeStorage={memoryStorage()} />);
    const group = screen.getByRole("radiogroup", { name: "Appearance" });
    expect(group).toBeInTheDocument();
    expect(screen.getAllByRole("radio")).toHaveLength(3);
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });
});
