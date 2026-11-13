/**
 * V3-03 — regression baseline for the UI defects confirmed in
 * `docs/V3/V3-02-defects.md`. Each test states today's defective behaviour, so
 * the defect is pinned by an assertion; the V3 task that fixes the cause
 * inverts the assertion in the same commit.
 */
import { describe, expect, it } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
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

/** D5: credentials are provider-scoped; pick one before visiting the screen. */
async function selectProvider(id: string) {
  await goTo("Provider");
  const select = screen.getByLabelText("Provider", { selector: "select" });
  await userEvent.selectOptions(select, id);
}

describe("V3-24 — a stored helper file is configured, never 'connected'", () => {
  it("labels an unverified helper as configured", () => {
    expect(connectionLabel(helperOnly)).toBe("configured");
  });

  it("labels a successful connection test as verified", () => {
    expect(connectionLabel(helperOnly, { lastTest: { ok: true } })).toBe("verified");
  });

  it("labels a failed connection test as unavailable", () => {
    expect(connectionLabel(helperOnly, { lastTest: { ok: false } })).toBe("unavailable");
  });

  it("labels a routed configuration with a dead helper as proxy stopped", () => {
    expect(connectionLabel(routed, { proxyRunning: false })).toBe("proxy stopped");
    expect(connectionLabel(routed, { proxyRunning: true })).toBe("routed");
  });

  it("shows the honest state in the header", async () => {
    render(<App ipc={mockWith(helperOnly)} />);
    expect(await screen.findByText("mode: configured")).toBeInTheDocument();
    expect(screen.queryByText("mode: connected")).not.toBeInTheDocument();
  });
});

describe("V3-26 — the credentials screen shows the path the backend reports", () => {
  it("uses the backend value when it is available", async () => {
    render(
      <App
        ipc={mockWith(helperOnly, {
          config_paths: () =>
            Promise.resolve({
              home: "/Users/x",
              secretsDir: "/Users/x/.claude-config-manager/secrets",
              helpersDir: "/Users/x/.claude-config-manager/helpers",
              routingFile: "/Users/x/.claude-config-manager/routing.json",
              settingsFile: "/Users/x/.claude/settings.json",
              helperState: "/Users/x/.claude-config-manager/proxy-helper.json",
            }),
        })}
      />,
    );
    await selectProvider("anthropic");
    await goTo("Credentials");
    expect(
      await screen.findByText("/Users/x/.claude-config-manager/secrets/"),
    ).toBeInTheDocument();
    expect(screen.queryByText(/Library\/Application Support/)).not.toBeInTheDocument();
  });

  it("falls back to the documented default instead of inventing a path", async () => {
    render(<App ipc={mockWith(helperOnly)} />);
    await selectProvider("anthropic");
    await goTo("Credentials");
    expect(await screen.findByText("~/.claude-config-manager/secrets/")).toBeInTheDocument();
  });
});

describe("V3-25 — routed mode is never replaced by a single click", () => {
  it("offers routed actions instead of the direct Save & Apply", async () => {
    render(<App ipc={mockWith(routed)} />);
    expect(await screen.findByRole("button", { name: "Apply routed mode" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Save & Apply" })).not.toBeInTheDocument();
  });

  it("asks for confirmation before switching back to direct mode", async () => {
    const calls: string[] = [];
    const ipc = mockWith(routed, {
      save_and_apply: () => {
        calls.push("save_and_apply");
        return Promise.resolve({
          settingsPath: "/Users/x/.claude/settings.json",
          backupFileName: null,
          helperPath: "/Users/x/.claude-config-manager/helpers/anthropic.sh",
          appliedKeys: ["model"],
        });
      },
    });
    render(<App ipc={ipc} />);
    await userEvent.click(await screen.findByRole("button", { name: "Switch to direct…" }));
    // Nothing has been written yet: the confirm bar is the guard.
    expect(calls).toEqual([]);
    expect(screen.getByText(/Switch Claude Code back to a direct provider/)).toBeInTheDocument();

    await goTo("Provider");
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "anthropic");
    await userEvent.type(screen.getByLabelText("API key", { selector: "input" }), "sk-fixture-v3");
    await userEvent.click(screen.getByRole("button", { name: "Switch to direct" }));
    await waitFor(() => expect(calls).toEqual(["save_and_apply"]));
  });
});

describe("V3-27 — the routing screen keeps its own credential list", () => {
  it("asks for the routing provider's credentials without touching the credentials screen", async () => {
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
    expect(requested).toContain("deepseek");
  });
});
