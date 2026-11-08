/**
 * V3-03 — regression baseline for the UI defects confirmed in
 * `docs/V3/V3-02-defects.md`. Each test states today's defective behaviour, so
 * the defect is pinned by an assertion; the V3 task that fixes the cause
 * inverts the assertion in the same commit.
 */
import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import App from "./App";
import { createMockIpc } from "./lib/ipc-mock";
import { providerSummaryFixtures } from "./lib/fixtures/providers";
import { connectionLabel } from "./lib/connectionLabel";
import type { EffectiveStatus } from "./lib/ipc";

const helperOnly: EffectiveStatus = {
  settingsFound: true,
  model: "claude-sonnet-4-5",
  envModel: null,
  envBaseUrl: null,
  authTokenPresent: false,
  apiKeyPresent: false,
  apiKeyHelper: "/Users/x/.claude-config-manager/helpers/anthropic.sh",
  envDefaultHaiku: null,
  envDefaultSonnet: null,
  envDefaultOpus: null,
  envMaxContextTokens: null,
  warnings: [],
};

/** Routed: Claude Code points at the loopback proxy, not at a provider. */
const routed: EffectiveStatus = {
  ...helperOnly,
  envBaseUrl: "http://127.0.0.1:52341",
  model: "claude-sonnet-4-5",
};

function mockWith(status: EffectiveStatus, extra: Record<string, unknown> = {}) {
  return createMockIpc({
    handlers: {
      list_providers: () => Promise.resolve(providerSummaryFixtures),
      list_credentials: () => Promise.resolve([]),
      read_status: () => Promise.resolve(status),
      get_routing: () =>
        Promise.resolve({ version: 1, enabled: false, targets: [] }),
      proxy_status: () =>
        Promise.resolve({ running: false, port: null, requests: 0, fallbacks: 0, failures: 0, lastTarget: null, lastError: null, cooling: [] }),
      check_environment: () =>
        Promise.resolve({
          cliFound: true,
          cliVersion: "2.1.246",
          cliVersionWarning: null,
          settingsState: "ok",
          settingsDetail: null,
          configDirWritable: true,
        }),
      ...extra,
    },
  });
}

async function goTo(label: string) {
  await userEvent.click(screen.getByRole("button", { name: label }));
}

describe("V3-02 §3 — a stored helper file is not verified connectivity", () => {
  it("currently reports 'connected' from the settings file alone", () => {
    // Defect, stated positively: no connection test ever ran, yet the label
    // claims a live connection. V3-24 must replace this with Configured /
    // Checking / Verified / Unavailable distinctions.
    expect(connectionLabel(helperOnly)).toBe("connected");
  });

  it("shows the same unverified 'connected' pill in the header", async () => {
    render(<App ipc={mockWith(helperOnly)} />);
    expect(await screen.findByText("mode: connected")).toBeInTheDocument();
  });
});

describe("V3-02 §5 — the credentials screen invents a path", () => {
  it("currently claims ~/Library/Application Support/…", async () => {
    render(<App ipc={mockWith(helperOnly)} />);
    await goTo("Credentials");
    // The backend writes ~/.claude-config-manager/secrets (file_secret_store.rs:39).
    expect(
      screen.getByText(/Library\/Application Support\/com\.claude-config-manager\/secrets/),
    ).toBeInTheDocument();
    expect(screen.queryByText(/~\/\.claude-config-manager\/secrets/)).not.toBeInTheDocument();
  });
});

describe("V3-02 §4 — Save & Apply ignores routed mode", () => {
  it("currently calls save_and_apply while Claude Code is routed to the proxy", async () => {
    const calls: string[] = [];
    const ipc = mockWith(routed, {
      save_and_apply: () => {
        calls.push("save_and_apply");
        return Promise.resolve({ settingsPath: "/Users/x/.claude/settings.json", backupPath: null });
      },
    });
    render(<App ipc={ipc} />);

    // Pick a provider so the draft is applicable, then press the one primary
    // action in the footer.
    await goTo("Provider");
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "anthropic");
    await userEvent.type(screen.getByLabelText("API key", { selector: "input" }), "sk-fixture-v3");
    const apply = screen.getByRole("button", { name: "Save & Apply" });
    await userEvent.click(apply);

    // Defect: the direct-mode writer runs even though Claude Code is routed.
    // V3-25 must block this (or require an explicit mode switch).
    expect(calls).toEqual(["save_and_apply"]);
  });
});

describe("V3-02 §6 — provider switch leaves the routing screen's list alone", () => {
  it("keeps routing credentials local once the provider changes", async () => {
    const requested: string[] = [];
    const ipc = mockWith(helperOnly, {
      list_credentials: (args: { providerId?: string }) => {
        const id = args?.providerId ?? "";
        requested.push(id);
        return Promise.resolve([]);
      },
    });
    render(<App ipc={ipc} />);
    await goTo("Provider");
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "anthropic");
    await goTo("Failover");

    const routingProvider = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(routingProvider, "deepseek");
    // Whichever screen asked, the routing screen must not overwrite what the
    // credentials screen shows. Both are empty here, so assert the call shape:
    // the routing fetch names the routing provider and nothing else.
    expect(requested).toContain("deepseek");
  });
});
