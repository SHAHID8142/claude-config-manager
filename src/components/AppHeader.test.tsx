import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
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
    render(<AppHeader status={status()} />);
    expect(screen.getByRole("heading", { name: "Claude Config Manager" })).toBeInTheDocument();
    expect(screen.getByLabelText("Connection mode")).toHaveTextContent("mode: connected");
  });

});
