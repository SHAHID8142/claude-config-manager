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

describe("connectionLabel (V3-24: a file on disk is configured, not connected)", () => {
  it("distinguishes checking, unconfigured, unverified, plaintext and routed", () => {
    expect(connectionLabel(null)).toBe("checking");
    expect(connectionLabel(status({ settingsFound: false }))).toBe("not configured");
    // A helper script proves a file exists, not that the provider accepted it.
    expect(connectionLabel(status())).toBe("configured");
    expect(connectionLabel(status(), { lastTest: { ok: true } })).toBe("verified");
    expect(connectionLabel(status(), { lastTest: { ok: false } })).toBe("unavailable");
    expect(connectionLabel(status({ apiKeyHelper: null, apiKeyPresent: true }))).toBe(
      "needs attention",
    );
    expect(connectionLabel(status({ apiKeyHelper: null }))).toBe("needs attention");
  });

  it("says the proxy is stopped when Claude Code points at it and nothing answers", () => {
    const routed = status({ envBaseUrl: "http://127.0.0.1:54321" });
    expect(connectionLabel(routed, { proxyRunning: false })).toBe("proxy stopped");
    expect(connectionLabel(routed, { proxyRunning: true })).toBe("routed");
  });
});

describe("AppHeader", () => {
  it("shows the title, icon and state pill", () => {
    render(<AppHeader status={status()} />);
    expect(screen.getByRole("heading", { name: "Claude Config Manager" })).toBeInTheDocument();
    expect(screen.getByLabelText("Connection mode")).toHaveTextContent("mode: configured");
  });
});
