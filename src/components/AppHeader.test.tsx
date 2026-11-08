import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AppHeader } from "./AppHeader";
import { connectionLabel } from "../lib/connectionLabel";
import type { EffectiveStatus } from "../lib/ipc";

function status(overrides: Partial<EffectiveStatus> = {}): EffectiveStatus {
  return {
    settingsFound: true,
    model: "m",
    envModel: null,
    envBaseUrl: "https://openrouter.ai/api",
    authTokenPresent: false,
    apiKeyPresent: false,
    apiKeyHelper: "/Users/u/.claude-config-manager/helpers/openrouter-default.sh",
    envDefaultHaiku: null,
    envDefaultSonnet: null,
    envDefaultOpus: null,
    envMaxContextTokens: null,
    warnings: [],
    ...overrides,
  };
}

function memoryStorage(initial: Record<string, string> = {}) {
  const map = new Map(Object.entries(initial));
  return {
    getItem: (k: string) => map.get(k) ?? null,
    setItem: (k: string, v: string) => void map.set(k, v),
    dump: () => Object.fromEntries(map),
  };
}

describe("connectionLabel", () => {
  it("distinguishes not-configured, helper, plaintext and missing credit", () => {
    expect(connectionLabel(null)).toBe("reading…");
    expect(connectionLabel(status({ settingsFound: false }))).toBe("not configured");
    expect(connectionLabel(status())).toBe("connected");
    expect(connectionLabel(status({ apiKeyHelper: null, apiKeyPresent: true }))).toBe(
      "connected (plaintext)",
    );
    expect(connectionLabel(status({ apiKeyHelper: null }))).toBe("no credential");
  });
});

describe("AppHeader", () => {
  it("shows the title, icon and state pill", () => {
    render(<AppHeader status={status()} themeStorage={memoryStorage()} />);
    expect(screen.getByRole("heading", { name: "Claude Config Manager" })).toBeInTheDocument();
    expect(screen.getByLabelText("Connection mode")).toHaveTextContent("mode: connected");
  });

  it("opens the appearance menu from the keyboard and switches theme", async () => {
    const user = userEvent.setup();
    const storage = memoryStorage({ "ccm.theme": "system" });
    render(<AppHeader status={status()} themeStorage={storage} />);

    const button = screen.getByRole("button", { name: "Settings" });
    expect(button).toHaveAttribute("aria-expanded", "false");
    button.focus();
    await user.keyboard("{Enter}");
    expect(button).toHaveAttribute("aria-expanded", "true");

    const dialog = screen.getByRole("dialog", { name: "Appearance" });
    expect(dialog).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "System" })).toHaveAttribute("aria-checked", "true");

    await user.click(screen.getByRole("radio", { name: "Dark" }));
    expect(screen.getByRole("radio", { name: "Dark" })).toHaveAttribute("aria-checked", "true");
    expect(storage.dump()["ccm.theme"]).toBe("dark");
    expect(document.documentElement.classList.contains("dark")).toBe(true);

    await user.keyboard("{Escape}");
    expect(screen.queryByRole("dialog", { name: "Appearance" })).not.toBeInTheDocument();
  });
});
