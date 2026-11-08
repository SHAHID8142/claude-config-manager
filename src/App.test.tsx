import { describe, expect, it } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import App from "./App";
import { createMockIpc } from "./lib/ipc-mock";
import { providerSummaryFixtures } from "./lib/fixtures/providers";
import type { EffectiveStatus, ModelEntry } from "./lib/ipc";

const statusFixture: EffectiveStatus = {
  settingsFound: true,
  model: "claude-sonnet-4-5",
  envModel: null,
  envBaseUrl: null,
  authTokenPresent: false,
  apiKeyPresent: false,
  apiKeyHelper: "/app/helpers/anthropic.sh",
  warnings: [],
};

function mockWithProviders(status = statusFixture) {
  return createMockIpc({
    handlers: {
      list_providers: () => Promise.resolve(providerSummaryFixtures),
      read_status: () => Promise.resolve(status),
      check_environment: () =>
        Promise.resolve({
          cliFound: true,
          cliVersion: "2.1.246",
          cliVersionWarning: null,
          settingsState: "ok",
          settingsDetail: null,
          configDirWritable: true,
        }),
    },
  });
}

describe("App shell (T12/T14 regression)", () => {
  it("renders the app title and all PRD sections", async () => {
    render(<App ipc={mockWithProviders()} />);
    expect(screen.getByRole("heading", { name: "Claude Config Manager" })).toBeInTheDocument();
    for (const section of ["Active configuration", "Provider", "Model", "Actions"]) {
      expect(screen.getByRole("region", { name: section })).toBeInTheDocument();
    }
    expect(await screen.findByText(/mode: direct \(Keychain helper\)/)).toBeInTheDocument();
  });

  it("keeps unwired controls disabled with honest placeholders", async () => {
    render(<App ipc={mockWithProviders()} />);
    expect(screen.getByLabelText("API key", { selector: "input" })).toBeDisabled();
    expect(screen.getByLabelText("Model", { selector: "input" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Test Connection" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Save & Apply" })).toBeDisabled();
    // Show/hide toggle follows the field's disabled state (T20)
    expect(screen.getByRole("button", { name: "Show" })).toBeDisabled();
  });

  it("enables the masked API key field once a provider is selected (T20)", async () => {
    render(<App ipc={mockWithProviders()} />);
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "anthropic");
    const keyInput = screen.getByLabelText("API key", { selector: "input" });
    expect(keyInput).toBeEnabled();
    expect(keyInput.getAttribute("type")).toBe("password");
    const showBtn = screen.getByRole("button", { name: "Show" });
    expect(showBtn).toBeEnabled();
    await userEvent.click(showBtn);
    expect(screen.getByRole("button", { name: "Hide" })).toHaveAttribute("aria-pressed", "true");
    const revealed = screen.getByLabelText("API key", { selector: "input" });
    expect(revealed.getAttribute("type")).toBe("text");
    await userEvent.type(revealed, "sk-test-fixture");
    expect((revealed as HTMLInputElement).value).toBe("sk-test-fixture");
  });

  it("links field hints via aria-describedby", async () => {
    render(<App ipc={mockWithProviders()} />);
    const apiKey = screen.getByLabelText("API key", { selector: "input" });
    expect(apiKey).toHaveAttribute("aria-describedby", "api-key-input-hint");
  });
});

describe("Provider selector (T18)", () => {
  it("loads and lists builtin providers from the catalog", async () => {
    const ipc = mockWithProviders();
    render(<App ipc={ipc} />);
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await waitFor(() => expect(select).toBeEnabled());
    const options = Array.from(select.querySelectorAll("option")).map((o) => o.textContent);
    expect(options).toEqual([
      "Select a provider",
      "Anthropic",
      "OpenRouter",
      "DeepSeek",
      "Z.AI",
      "NVIDIA NIM",
      "Custom…",
    ]);
    // read_status fires first (status effect), then the catalog loads.
    expect(ipc.calls.map((c) => c.cmd).sort()).toEqual([
      "check_environment",
      "list_providers",
      "read_status",
    ]);
  });

  it("selecting a preset shows capability and connection badges (T25)", async () => {
    const ipc = mockWithProviders();
    render(<App ipc={ipc} />);
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "anthropic");
    expect(screen.getByText(/capabilities verified/)).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "(source)" })).toHaveAttribute(
      "href",
      "https://platform.claude.com/docs/en/api/models-list",
    );
    expect(screen.getByText(/direct connection/)).toBeInTheDocument();
    // Preset endpoint shown read-only and disabled.
    const endpoint = screen.getByLabelText("Endpoint URL", { selector: "input" });
    expect(endpoint).toBeDisabled();
    expect(endpoint).toHaveValue("https://api.anthropic.com");
  });

  it("shows the bridge-required badge for OpenAI-only providers (T25)", async () => {
    const ipc = mockWithProviders();
    render(<App ipc={ipc} />);
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "nvidia-nim");
    expect(screen.getByText(/bridge required/)).toBeInTheDocument();
    expect(screen.queryByText(/direct connection/)).not.toBeInTheDocument();
    // No Anthropic base URL to display for OpenAI-only providers.
    expect(screen.queryByLabelText("Endpoint URL", { selector: "input" })).not.toBeInTheDocument();
  });

  it("selecting Custom reveals custom name/endpoint entry", async () => {
    const ipc = mockWithProviders();
    render(<App ipc={ipc} />);
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "__custom__");
    const nameInput = screen.getByLabelText("Custom provider name", { selector: "input" });
    const endpointInput = screen.getByLabelText("Custom endpoint URL", { selector: "input" });
    expect(nameInput).toBeEnabled();
    expect(endpointInput).toBeEnabled();
    await userEvent.type(nameInput, "My gateway");
    await userEvent.type(endpointInput, "https://gw.example.com");
    expect(nameInput).toHaveValue("My gateway");
    expect(endpointInput).toHaveValue("https://gw.example.com");
    // https remote → no warning shown
    expect(screen.queryByText(/insecure connection/i)).not.toBeInTheDocument();
  });

  it("warns insecure http endpoints live while typing (T19)", async () => {
    const ipc = mockWithProviders();
    render(<App ipc={ipc} />);
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "__custom__");
    const endpointInput = screen.getByLabelText("Custom endpoint URL", { selector: "input" });
    await userEvent.type(endpointInput, "http://evil.example.com");
    const warning = screen.getByText(/Refused: credentials would be sent over an insecure/i);
    expect(warning).toHaveAttribute("aria-live", "polite");
  });

  it("notes local endpoints without failing them (T19)", async () => {
    const ipc = mockWithProviders();
    render(<App ipc={ipc} />);
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "__custom__");
    const endpointInput = screen.getByLabelText("Custom endpoint URL", { selector: "input" });
    await userEvent.type(endpointInput, "https://localhost:8080");
    expect(
      screen.getByText(/Local endpoint — API traffic stays on this machine/i),
    ).toBeInTheDocument();
  });

  it("shows an error banner when the catalog fails to load", async () => {
    const ipc = createMockIpc({
      failures: { list_providers: { kind: "not_available", message: "backend offline" } },
    });
    render(<App ipc={ipc} />);
    await waitFor(() => {
      expect(screen.getByRole("alert")).toBeInTheDocument();
    });
    expect(screen.getByText("backend offline")).toBeInTheDocument();
  });
});

describe("Save & Apply (T33)", () => {
  const outcomeFixture = {
    settingsPath: "/tmp/fixture/.claude/settings.json",
    backupFileName: "settings-backup-1700000000000000000.json",
    helperPath: "/tmp/fixture/helpers/anthropic.sh",
    appliedKeys: ["model", "apiKeyHelper", "credential:anthropic"],
  };

  function mockWithApply() {
    return createMockIpc({
      handlers: {
        list_providers: () => Promise.resolve(providerSummaryFixtures),
        save_and_apply: (args) => {
          expect(args?.providerId).toBe("anthropic");
          expect(args?.apiKey).toBe("sk-test-fixture-key");
          expect(args?.baseUrl).toBeNull();
          return Promise.resolve(outcomeFixture);
        },
      },
    });
  }

  it("applies for a preset: button enables with key, succeeds with banner", async () => {
    const ipc = mockWithApply();
    render(<App ipc={ipc} />);
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "anthropic");

    const keyInput = screen.getByLabelText("API key", { selector: "input" });
    const applyBtn = screen.getByRole("button", { name: "Save & Apply" });
    // Disabled until a key is entered.
    expect(applyBtn).toBeDisabled();
    await userEvent.type(keyInput, "sk-test-fixture-key");
    expect(applyBtn).toBeEnabled();

    await userEvent.click(applyBtn);
    await waitFor(() => {
      expect(screen.getByText("Configuration applied and verified.")).toBeInTheDocument();
    });
    expect(screen.getByText(/Backup: settings-backup-/)).toBeInTheDocument();
    expect(screen.getByText(/never written into/)).toBeInTheDocument();
  });

  it("shows an error banner when apply fails, without leaking the key", async () => {
    const ipc = createMockIpc({
      handlers: {
        list_providers: () => Promise.resolve(providerSummaryFixtures),
        save_and_apply: () =>
          Promise.reject({
            kind: "io",
            message: "write verification failed",
          }),
      },
    });
    render(<App ipc={ipc} />);
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "anthropic");
    await userEvent.type(screen.getByLabelText("API key", { selector: "input" }), "sk-secret-x");
    await userEvent.click(screen.getByRole("button", { name: "Save & Apply" }));
    await waitFor(() => {
      expect(screen.getByText("Save & Apply failed.")).toBeInTheDocument();
    });
    expect(screen.getByText("write verification failed")).toBeInTheDocument();
    expect(document.body.textContent).not.toContain("sk-secret-x");
  });

  it("passes the compat preset base URL and custom endpoint through", async () => {
    const calls: Array<Record<string, unknown>> = [];
    const ipc = createMockIpc({
      handlers: {
        list_providers: () => Promise.resolve(providerSummaryFixtures),
        save_and_apply: (args) => {
          calls.push(args ?? {});
          return Promise.resolve({ ...outcomeFixture, appliedKeys: [] });
        },
      },
    });
    render(<App ipc={ipc} />);
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "deepseek");
    await userEvent.type(screen.getByLabelText("API key", { selector: "input" }), "sk-x");
    await userEvent.click(screen.getByRole("button", { name: "Save & Apply" }));
    await waitFor(() => expect(calls).toHaveLength(1));
    expect(calls[0].providerId).toBe("deepseek");
    expect(calls[0].baseUrl).toBe("https://api.deepseek.com/anthropic");

    // Custom path
    await userEvent.selectOptions(select, "__custom__");
    await userEvent.type(
      screen.getByLabelText("Custom endpoint URL", { selector: "input" }),
      "https://gw.example.com",
    );
    await userEvent.click(screen.getByRole("button", { name: "Save & Apply" }));
    await waitFor(() => expect(calls).toHaveLength(2));
    expect(calls[1].providerId).toBe("custom");
    expect(calls[1].baseUrl).toBe("https://gw.example.com");
  });
});

describe("Effective status + override warnings (T34)", () => {
  it("shows first-run state when no settings file exists", async () => {
    const ipc = mockWithProviders({
      ...statusFixture,
      settingsFound: false,
      model: null,
      apiKeyHelper: null,
    });
    render(<App ipc={ipc} />);
    expect(await screen.findByText(/No settings file yet/)).toBeInTheDocument();
    expect(await screen.findByText(/mode: not configured/)).toBeInTheDocument();
  });

  it("warns when apiKeyHelper is shadowed by a plaintext env key", async () => {
    const ipc = mockWithProviders({
      ...statusFixture,
      apiKeyPresent: true,
      warnings: [
        { helperShadowedByEnv: null },
        { plaintextKeyInEnv: { which: "ANTHROPIC_API_KEY" } },
      ],
    });
    render(<App ipc={ipc} />);
    expect(
      await screen.findByText(/apiKeyHelper is shadowed by a plaintext env credential/),
    ).toBeInTheDocument();
    expect(await screen.findByText(/ANTHROPIC_API_KEY is stored in plaintext/)).toBeInTheDocument();
  });

  it("warns on model conflict with env override", async () => {
    const ipc = mockWithProviders({
      ...statusFixture,
      model: "claude-x",
      envModel: "claude-y",
      warnings: [{ modelConflict: { settingModel: "claude-x", envModel: "claude-y" } }],
    });
    render(<App ipc={ipc} />);
    expect(
      await screen.findByText(
        /Model conflict: settings say "claude-x" but ANTHROPIC_MODEL says "claude-y" \(env wins\)/,
      ),
    ).toBeInTheDocument();
  });

  it("discloses current endpoint without treating it as a secret", async () => {
    const ipc = mockWithProviders({
      ...statusFixture,
      envBaseUrl: "http://127.0.0.1:31415",
    });
    render(<App ipc={ipc} />);
    expect(await screen.findByText(/Endpoint:/)).toBeInTheDocument();
    expect(screen.getByText("http://127.0.0.1:31415")).toBeInTheDocument();
  });

  it("refreshes status after a successful apply", async () => {
    let statusCallCount = 0;
    const ipc = createMockIpc({
      handlers: {
        list_providers: () => Promise.resolve(providerSummaryFixtures),
        read_status: () => {
          statusCallCount += 1;
          return Promise.resolve(statusFixture);
        },
        save_and_apply: () =>
          Promise.resolve({
            settingsPath: "/tmp/x",
            backupFileName: "settings-backup-1.json",
            helperPath: "/tmp/h",
            appliedKeys: ["model"],
          }),
      },
    });
    render(<App ipc={ipc} />);
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await waitFor(() => expect(statusCallCount).toBe(1));
    await userEvent.selectOptions(select, "anthropic");
    await userEvent.type(screen.getByLabelText("API key", { selector: "input" }), "sk-x");
    await userEvent.click(screen.getByRole("button", { name: "Save & Apply" }));
    await waitFor(() => expect(statusCallCount).toBeGreaterThanOrEqual(2));
  });
});

describe("Restore last backup (T35)", () => {
  it("restores and shows the outcome with pre-restore copy", async () => {
    const ipc = createMockIpc({
      handlers: {
        list_providers: () => Promise.resolve(providerSummaryFixtures),
        read_status: () => Promise.resolve(statusFixture),
        restore_last_backup: () =>
          Promise.resolve({
            restoredFrom: "settings-backup-1700000000000000000.json",
            preRestoreBackup: "settings-backup-1799999999999999999.json",
          }),
      },
    });
    render(<App ipc={ipc} />);
    const btn = await screen.findByRole("button", { name: "Restore last backup" });
    await userEvent.click(btn);
    await waitFor(() => {
      expect(screen.getByText("Settings restored from the latest backup.")).toBeInTheDocument();
    });
    expect(screen.getByText(/Restored: settings-backup-1700000000000000000/)).toBeInTheDocument();
    expect(
      screen.getByText(/pre-restore copy: settings-backup-1799999999999999999/),
    ).toBeInTheDocument();
  });

  it("shows restore errors cleanly (e.g. no backups)", async () => {
    const ipc = createMockIpc({
      handlers: {
        list_providers: () => Promise.resolve(providerSummaryFixtures),
        read_status: () => Promise.resolve(statusFixture),
        restore_last_backup: () =>
          Promise.reject({ kind: "io", message: "no backups available to restore" }),
      },
    });
    render(<App ipc={ipc} />);
    const btn = await screen.findByRole("button", { name: "Restore last backup" });
    await userEvent.click(btn);
    await waitFor(() => {
      expect(screen.getByText("Restore failed.")).toBeInTheDocument();
    });
    expect(screen.getByText("no backups available to restore")).toBeInTheDocument();
  });
});

describe("Environment diagnostics (T36)", () => {
  it("warns when the Claude Code CLI is missing", async () => {
    const ipc = createMockIpc({
      handlers: {
        list_providers: () => Promise.resolve(providerSummaryFixtures),
        read_status: () => Promise.resolve(statusFixture),
        check_environment: () =>
          Promise.resolve({
            cliFound: false,
            cliVersion: null,
            cliVersionWarning: null,
            settingsState: "first_run",
            settingsDetail: null,
            configDirWritable: true,
          }),
      },
    });
    render(<App ipc={ipc} />);
    expect(await screen.findByText("Claude Code CLI not found on PATH.")).toBeInTheDocument();
  });

  it("flags malformed settings as never-overwrite", async () => {
    const ipc = createMockIpc({
      handlers: {
        list_providers: () => Promise.resolve(providerSummaryFixtures),
        read_status: () => Promise.resolve(statusFixture),
        check_environment: () =>
          Promise.resolve({
            cliFound: true,
            cliVersion: "2.1.246",
            cliVersionWarning: null,
            settingsState: "malformed_json",
            settingsDetail: "settings file contains invalid JSON",
            configDirWritable: true,
          }),
      },
    });
    render(<App ipc={ipc} />);
    expect(
      await screen.findByText(/settings file has a problem this app will not overwrite/i),
    ).toBeInTheDocument();
    expect(screen.getByText("settings file contains invalid JSON")).toBeInTheDocument();
  });

  it("shows no environment banners when everything is healthy", async () => {
    const ipc = mockWithProviders();
    render(<App ipc={ipc} />);
    await waitFor(() => {
      expect(screen.queryByText("Claude Code CLI not found on PATH.")).not.toBeInTheDocument();
    });
    expect(screen.queryByText(/settings file has a problem/i)).not.toBeInTheDocument();
  });
});

describe("Model discovery states (T39)", () => {
  function mockWithModels(models: ModelEntry[] | { fail: string }) {
    return createMockIpc({
      handlers: {
        list_providers: () => Promise.resolve(providerSummaryFixtures),
        read_status: () => Promise.resolve(statusFixture),
        check_environment: () =>
          Promise.resolve({
            cliFound: true,
            cliVersion: "2.1.246",
            cliVersionWarning: null,
            settingsState: "ok",
            settingsDetail: null,
            configDirWritable: true,
          }),
        discover_models: () =>
          Array.isArray(models)
            ? Promise.resolve(models)
            : Promise.reject({ kind: "io", message: models.fail }),
      },
    });
  }

  it("fetch button disabled without provider, enabled with one; success shows count", async () => {
    const ipc = mockWithModels([
      { id: "model-a", displayName: "Model A", capabilityStatus: "unknown" },
      { id: "model-b", displayName: null, capabilityStatus: "unknown" },
    ]);
    render(<App ipc={ipc} />);
    const fetchBtn = await screen.findByRole("button", { name: "Fetch models" });
    expect(fetchBtn).toBeDisabled();
    const select = screen.getByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "openrouter");
    expect(fetchBtn).toBeEnabled();
    await userEvent.click(fetchBtn);
    expect(await screen.findByText(/2 models discovered/)).toBeInTheDocument();
    // Never auto-selected: the model input stays disabled/empty.
    expect(screen.getByLabelText("Model", { selector: "input" })).toBeDisabled();
  });

  it("shows an error banner on discovery failure with retry available", async () => {
    const ipc = mockWithModels({ fail: "catalog endpoint returned HTTP 403" });
    render(<App ipc={ipc} />);
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "openrouter");
    await userEvent.click(await screen.findByRole("button", { name: "Fetch models" }));
    expect(await screen.findByText("Model discovery failed.")).toBeInTheDocument();
    expect(screen.getByText("catalog endpoint returned HTTP 403")).toBeInTheDocument();
    // Retry remains possible.
    expect(screen.getByRole("button", { name: "Fetch models" })).toBeEnabled();
  });

  it("shows an empty-catalog state without selecting anything", async () => {
    const ipc = mockWithModels([]);
    render(<App ipc={ipc} />);
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "openrouter");
    await userEvent.click(await screen.findByRole("button", { name: "Fetch models" }));
    expect(await screen.findByText(/Catalog returned no models/)).toBeInTheDocument();
    expect(screen.getByLabelText("Model", { selector: "input" })).toBeDisabled();
  });
});
