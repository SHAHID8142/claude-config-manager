import { describe, expect, it } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import App from "./App";
import { createMockIpc } from "./lib/ipc-mock";
import { providerSummaryFixtures } from "./lib/fixtures/providers";
import type { EffectiveStatus, ModelEntry } from "./lib/ipc";

/** Walk the sidebar to a screen (macOS settings shape: one panel at a time). */
async function goTo(label: string) {
  await userEvent.click(screen.getByRole("button", { name: label }));
}

const statusFixture: EffectiveStatus = {
  settingsFound: true,
  model: "claude-sonnet-4-5",
  envModel: null,
  envBaseUrl: null,
  authTokenPresent: false,
  apiKeyPresent: false,
  apiKeyHelper: "/app/helpers/anthropic.sh",
  envDefaultHaiku: null,
  envDefaultSonnet: null,
  envDefaultOpus: null,
  envMaxContextTokens: null,
  warnings: [],
};

function mockWithProviders(status = statusFixture) {
  return createMockIpc({
    handlers: {
      list_providers: () => Promise.resolve(providerSummaryFixtures),
      // V2-21: the credentials panel always asks once per provider switch.
      list_credentials: () => Promise.resolve([]),
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
    // Dashboard answers first…
    expect(screen.getByRole("region", { name: "Active configuration" })).toBeInTheDocument();
    expect(await screen.findByText(/direct \(credential helper\)/)).toBeInTheDocument();
    // …and each sidebar item owns exactly one screen.
    await goTo("Provider");
    expect(screen.getByRole("region", { name: "Provider" })).toBeInTheDocument();
    await goTo("Model");
    expect(screen.getByRole("region", { name: "Model" })).toBeInTheDocument();
    await goTo("Maintenance");
    expect(screen.getByRole("region", { name: "Actions" })).toBeInTheDocument();
  });

  it("keeps unwired controls disabled with honest placeholders", async () => {
    render(<App ipc={mockWithProviders()} />);
    await goTo("Provider");
    expect(screen.getByLabelText("API key", { selector: "input" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Test Connection" })).toBeDisabled();
    // Show/hide toggle follows the field's disabled state (T20)
    expect(screen.getByRole("button", { name: "Show" })).toBeDisabled();
    await goTo("Model");
    expect(screen.getByLabelText("Model", { selector: "input" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Save & Apply" })).toBeDisabled();
  });

  it("enables the masked API key field once a provider is selected (T20)", async () => {
    render(<App ipc={mockWithProviders()} />);
    await goTo("Provider");
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
    await goTo("Provider");
    const apiKey = screen.getByLabelText("API key", { selector: "input" });
    expect(apiKey).toHaveAttribute("aria-describedby", "api-key-input-hint");
  });
});

describe("Provider selector (T18)", () => {
  it("loads and lists builtin providers from the catalog", async () => {
    const ipc = mockWithProviders();
    render(<App ipc={ipc} />);
    await goTo("Provider");
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
    // Through the first paint only the shell probes run. The credential
    // commands are deliberately absent: "__none__" is a UI sentinel, not a
    // provider id, and asking the backend for it produced a red
    // "invalid provider id" banner on first launch.
    expect(ipc.calls.map((c) => c.cmd).sort()).toEqual([
      "check_environment",
      "config_paths",
      "get_routing",
      "helper_status",
      "list_backups",
      "list_credentials",
      "list_providers",
      "proxy_status",
      "read_status",
    ]);
  });

  it("changing only the model keeps Save enabled when a key is already stored", async () => {
    const ipc = mockWithProviders();
    // An empty stored-key answer is the default; this provider has one.
    const withStored = {
      invoke: <T,>(cmd: string, args?: Record<string, unknown>) =>
        cmd === "has_credential" ? Promise.resolve(true as T) : ipc.invoke<T>(cmd, args),
      calls: ipc.calls,
    } as typeof ipc;
    render(<App ipc={withStored} />);
    await goTo("Provider");
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await waitFor(() => expect(select).toBeEnabled());
    await userEvent.selectOptions(select, "openrouter");
    // No key typed: the stored credential is what authorizes the apply.
    const save = await screen.findByRole("button", { name: /save & apply/i });
    await waitFor(() => expect(save).toBeEnabled());
    expect(screen.getByText(/leave this blank to keep it/i)).toBeInTheDocument();
    const key = screen.getByLabelText("API key", { selector: "input" });
    expect(key).toHaveValue("");
  });

  it("keeps Save disabled with a blank key and no stored credential", async () => {
    const ipc = mockWithProviders();
    render(<App ipc={ipc} />);
    await goTo("Provider");
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await waitFor(() => expect(select).toBeEnabled());
    await userEvent.selectOptions(select, "openrouter");
    const save = screen.getByRole("button", { name: /save & apply/i });
    await waitFor(() => expect(save).toBeDisabled());
  });

  it("selecting a preset shows capability and connection badges (T25)", async () => {
    const ipc = mockWithProviders();
    render(<App ipc={ipc} />);
    await goTo("Provider");
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "anthropic");
    expect(screen.getByText(/capabilities verified/)).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "source" })).toHaveAttribute(
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
    await goTo("Provider");
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
    await goTo("Provider");
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
    await goTo("Provider");
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
    await goTo("Provider");
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
      handlers: { list_credentials: () => Promise.resolve([]) },
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
    await goTo("Provider");
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "anthropic");

    const keyInput = screen.getByLabelText("API key", { selector: "input" });
    const applyBtn = screen.getByRole("button", { name: "Save & Apply" });
    // Disabled until a key is entered.
    expect(applyBtn).toBeDisabled();
    await userEvent.type(keyInput, "sk-test-fixture-key");
    expect(applyBtn).toBeEnabled();
    // The promise made before saving: the key never lands in settings.json.
    expect(screen.getByText(/never written to logs or backups/i)).toBeInTheDocument();

    await userEvent.click(applyBtn);
    await goTo("Dashboard");
    await waitFor(() => {
      expect(screen.getByText("Configuration applied and verified.")).toBeInTheDocument();
    });
    expect(screen.getByText(/Backup: settings-backup-/)).toBeInTheDocument();
  });

  it("points out the 200k assumption for a non-Claude model id (V2-13)", async () => {
    const ipc = mockWithApply();
    render(<App ipc={ipc} />);
    await goTo("Provider");
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "openrouter");
    await goTo("Model");
    await userEvent.type(
      screen.getByLabelText("Model", { selector: "input" }),
      "nvidia/nemotron-x",
    );
    expect(screen.getByText(/will assume a 200k window/i)).toBeInTheDocument();
    // The tip is a real affordance: it opens the panel that can fix it.
    await userEvent.click(screen.getByRole("button", { name: /declare its real window/i }));
    expect(screen.getByLabelText(/context window/i)).toBeInTheDocument();
  });

  it("sends the declared context window for an unrecognized model id (V2-12b)", async () => {
    const ipc = mockWithApply();
    render(<App ipc={ipc} />);
    await goTo("Provider");
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "anthropic");
    await userEvent.type(
      screen.getByLabelText("API key", { selector: "input" }),
      "sk-test-fixture-key",
    );
    await goTo("Model");
    await userEvent.click(screen.getByRole("button", { name: /advanced settings/i }));
    const windowInput = screen.getByLabelText(/context window/i);
    // Digits only: a typed "1M" must never reach settings.json as a no-op value.
    await userEvent.type(windowInput, "1M000000");
    expect(windowInput).toHaveValue("1000000");
    await userEvent.click(screen.getByRole("button", { name: "Save & Apply" }));
    await waitFor(() => expect(ipc.calls.some((c) => c.cmd === "save_and_apply")).toBe(true));
    const call = ipc.calls.find((c) => c.cmd === "save_and_apply");
    expect(call?.args).toMatchObject({ maxContextTokens: 1000000, resetContextWindow: false });
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
    await goTo("Provider");
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "anthropic");
    await userEvent.type(screen.getByLabelText("API key", { selector: "input" }), "sk-secret-x");
    await userEvent.click(screen.getByRole("button", { name: "Save & Apply" }));
    await goTo("Dashboard");
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
    await goTo("Provider");
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
      warnings: ["helperShadowedByEnv", { plaintextKeyInEnv: { which: "ANTHROPIC_API_KEY" } }],
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
    expect(await screen.findByText("Endpoint")).toBeInTheDocument();
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
    await goTo("Provider");
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
    await goTo("Maintenance");
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
    await goTo("Maintenance");
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
      {
        id: "model-a",
        displayName: "Model A",
        contextWindow: null,
        capabilityStatus: "unknown",
        source: "https://openrouter.ai/api/v1/models",
      },
      {
        id: "model-b",
        displayName: null,
        contextWindow: null,
        capabilityStatus: "unknown",
        source: "https://openrouter.ai/api/v1/models",
      },
    ]);
    render(<App ipc={ipc} />);
    await goTo("Model");
    const fetchBtn = await screen.findByRole("button", { name: "Fetch models" });
    expect(fetchBtn).toBeDisabled();
    await goTo("Provider");
    await userEvent.selectOptions(
      screen.getByLabelText("Provider", { selector: "select" }),
      "openrouter",
    );
    await goTo("Model");
    // Re-query: the first screen was unmounted when the sidebar switched.
    const ready = await screen.findByRole("button", { name: "Fetch models" });
    await waitFor(() => expect(ready).toBeEnabled());
    await userEvent.click(ready);
    expect(await screen.findByLabelText("Discovered models")).toBeInTheDocument();
    // Never auto-selected: no option selected, input shows no value.
    expect(document.querySelectorAll("[aria-selected='true']")).toHaveLength(0);
    await goTo("Model");
    expect(screen.getByLabelText("Model", { selector: "input" })).toHaveValue("");
  });

  it("shows an error banner on discovery failure with retry available", async () => {
    const ipc = mockWithModels({ fail: "catalog endpoint returned HTTP 403" });
    render(<App ipc={ipc} />);
    await goTo("Provider");
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "openrouter");
    await goTo("Model");
    await userEvent.click(await screen.findByRole("button", { name: "Fetch models" }));
    expect(await screen.findByText("Model discovery failed.")).toBeInTheDocument();
    expect(screen.getByText("catalog endpoint returned HTTP 403")).toBeInTheDocument();
    // Retry remains possible.
    expect(screen.getByRole("button", { name: "Fetch models" })).toBeEnabled();
  });

  it("shows an empty-catalog state without selecting anything", async () => {
    const ipc = mockWithModels([]);
    render(<App ipc={ipc} />);
    await goTo("Provider");
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "openrouter");
    await goTo("Model");
    await userEvent.click(await screen.findByRole("button", { name: "Fetch models" }));
    expect(await screen.findByText(/Catalog returned no models/)).toBeInTheDocument();
  });
});

describe("Model selection (T40)", () => {
  function mockWithModels(models: ModelEntry[]) {
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
        discover_models: () => Promise.resolve(models),
      },
    });
  }

  const catalog: ModelEntry[] = [
    {
      id: "model-a",
      displayName: "Model A",
      contextWindow: null,
      capabilityStatus: "unknown",
      source: "https://openrouter.ai/api/v1/models",
    },
    {
      id: "model-b",
      displayName: null,
      contextWindow: null,
      capabilityStatus: "unknown",
      source: "https://openrouter.ai/api/v1/models",
    },
  ];

  it("explicit click selects a model and shows it in the Model field", async () => {
    const ipc = mockWithModels(catalog);
    render(<App ipc={ipc} />);
    await goTo("Provider");
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "openrouter");
    await goTo("Model");
    await userEvent.click(await screen.findByRole("button", { name: "Fetch models" }));
    await screen.findByLabelText("Discovered models");
    expect(document.querySelectorAll("[aria-selected='true']")).toHaveLength(0);
    await userEvent.click(screen.getByRole("option", { name: /model-a/ }));
    expect(await screen.findByLabelText("Model", { selector: "input" })).toHaveValue("model-a");
  });

  it("re-fetch resets any previous selection (no cross-provider inheritance)", async () => {
    const ipc = mockWithModels(catalog);
    render(<App ipc={ipc} />);
    await goTo("Provider");
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "openrouter");
    await goTo("Model");
    await userEvent.click(await screen.findByRole("button", { name: "Fetch models" }));
    await userEvent.click(await screen.findByRole("option", { name: /model-a/ }));
    await goTo("Model");
    expect(screen.getByLabelText("Model", { selector: "input" })).toHaveValue("model-a");
    await userEvent.click(screen.getByRole("button", { name: "Fetch models" }));
    await waitFor(() => {
      expect(screen.getByLabelText("Model", { selector: "input" })).toHaveValue("");
    });
  });
});

describe("Manual Model ID entry (T41)", () => {
  function baseMock(discoverOutcome: "none" | ModelEntry[]) {
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
          discoverOutcome === "none"
            ? Promise.reject({
                kind: "invalid_input",
                // Mirrors Rust `DiscoveryError::InvalidEndpoint` for Z.AI
                // (DiscoverySpec::None) surfaced through save/apply guards.
                message: "provider template declares no catalog endpoint",
              })
            : Promise.resolve(discoverOutcome),
      },
    });
  }

  it("is editable for a manual-entry provider (zai) and custom providers", async () => {
    const ipc = baseMock("none");
    render(<App ipc={ipc} />);
    await goTo("Provider");
    const select = await screen.findByLabelText("Provider", { selector: "select" });

    await userEvent.selectOptions(select, "zai");
    await goTo("Model");
    const modelInput = screen.getByLabelText("Model", { selector: "input" });
    expect(modelInput).toBeEnabled();
    await userEvent.type(modelInput, "glm-4.6");
    expect(modelInput).toHaveValue("glm-4.6");

    await goTo("Provider");
    await userEvent.selectOptions(
      screen.getByLabelText("Provider", { selector: "select" }),
      "__custom__",
    );
    await goTo("Model");
    // Custom: editable, and the previous provider's model was reset.
    expect(screen.getByLabelText("Model", { selector: "input" })).toHaveValue("");
    await userEvent.type(screen.getByLabelText("Model", { selector: "input" }), "my-model");
    expect(screen.getByLabelText("Model", { selector: "input" })).toHaveValue("my-model");
  });

  it("switching providers resets a previously entered model id", async () => {
    const ipc = baseMock("none");
    render(<App ipc={ipc} />);
    await goTo("Provider");
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "zai");
    await goTo("Model");
    await goTo("Provider");
    await userEvent.selectOptions(
      screen.getByLabelText("Provider", { selector: "select" }),
      "anthropic",
    );
    await goTo("Model");
    expect(screen.getByLabelText("Model", { selector: "input" })).toHaveValue("");
  });

  it("keeps manual entry available after a discovery failure (PRD F05)", async () => {
    const ipc = baseMock("none");
    render(<App ipc={ipc} />);
    await goTo("Provider");
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "zai");
    await goTo("Model");

    // The provider publishes no catalog: fetching surfaces a typed error…
    await userEvent.click(await screen.findByRole("button", { name: "Fetch models" }));
    expect(await screen.findByText("Model discovery failed.")).toBeInTheDocument();
    await goTo("Model");

    // …and the manual fallback still works (the point of T41).
    const modelInput = screen.getByLabelText("Model", { selector: "input" });
    expect(modelInput).toBeEnabled();
    expect(modelInput).not.toHaveAttribute("readonly");
    await userEvent.type(modelInput, "glm-4.6");
    expect(modelInput).toHaveValue("glm-4.6");
  });

  it("stays read-only while a populated catalog is displayed (T40 path)", async () => {
    const ipc = baseMock([
      {
        id: "m-1",
        displayName: null,
        contextWindow: null,
        capabilityStatus: "unknown",
        source: "https://openrouter.ai/api/v1/models",
      },
    ]);
    render(<App ipc={ipc} />);
    await goTo("Provider");
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "openrouter");
    await goTo("Model");
    await userEvent.click(await screen.findByRole("button", { name: "Fetch models" }));
    await screen.findByLabelText("Discovered models");
    await goTo("Model");
    expect(screen.getByLabelText("Model", { selector: "input" })).toHaveAttribute("readonly");
  });
});

describe("Model metadata provenance (T42)", () => {
  const CATALOG_URL = "https://openrouter.ai/api/v1/models";

  it("labels a typed model id as manual entry with unverified capabilities", async () => {
    render(<App ipc={mockWithProviders()} />);
    await goTo("Provider");
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "zai");
    await goTo("Model");
    await userEvent.type(screen.getByLabelText("Model", { selector: "input" }), "glm-4.6");

    expect(screen.getByText(/Source: manual entry/)).toBeInTheDocument();
    expect(screen.getByText(/capability data unverified/)).toBeInTheDocument();
    // The manual label never claims a catalog served this id.
    expect(screen.queryByText(CATALOG_URL)).not.toBeInTheDocument();
  });

  it("shows the verified catalog as provenance once discovery succeeds", async () => {
    const ipc = mockWithProviders();
    ipc.on("discover_models", () =>
      Promise.resolve([
        {
          id: "m-1",
          displayName: null,
          contextWindow: null,
          capabilityStatus: "unknown",
          source: CATALOG_URL,
        },
      ]),
    );
    render(<App ipc={ipc} />);
    await goTo("Provider");
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "openrouter");
    await goTo("Model");
    await userEvent.click(await screen.findByRole("button", { name: "Fetch models" }));

    await screen.findByLabelText("Discovered models");
    expect(screen.getByText(CATALOG_URL)).toBeInTheDocument();
    expect(screen.getByText(/capability data unverified/)).toBeInTheDocument();
    // No manual-entry label while a catalog is displayed.
    expect(screen.queryByText(/Source: manual entry/)).not.toBeInTheDocument();
  });

  it("does not invent capability values in the manual path", async () => {
    render(<App ipc={mockWithProviders()} />);
    await goTo("Provider");
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "zai");
    await goTo("Model");
    await userEvent.type(screen.getByLabelText("Model", { selector: "input" }), "glm-4.6");
    // "context window" is now a real managed setting (V2-12b), so the honesty
    // check is about invented capability CLAIMS, not about the word itself.
    expect(document.body.textContent).not.toMatch(/vision|reasoning|multimodal/i);
    expect(document.body.textContent).toMatch(/capability data unverified/i);
  });
});

describe("Model selection validation + Save & Apply integration (T43)", () => {
  const outcomeFixture = {
    settingsPath: "/tmp/fixture/.claude/settings.json",
    backupFileName: "settings-backup-1700000000000000000.json",
    helperPath: "/tmp/fixture/helpers/anthropic.sh",
    appliedKeys: ["model", "apiKeyHelper"],
  };

  function mockCapturing() {
    const calls: Array<Record<string, unknown>> = [];
    const ipc = createMockIpc({
      handlers: {
        list_providers: () => Promise.resolve(providerSummaryFixtures),
        save_and_apply: (args) => {
          calls.push(args ?? {});
          return Promise.resolve(outcomeFixture);
        },
      },
    });
    return { ipc, calls };
  }

  it("warns when the login shell exports a credential that overrides the app", async () => {
    const ipc = createMockIpc({
      handlers: {
        list_providers: () => Promise.resolve(providerSummaryFixtures),
        read_status: () => Promise.resolve(statusFixture),
        check_environment: () =>
          Promise.resolve({
            cliFound: true,
            cliVersion: "2.1.319",
            cliVersionWarning: null,
            settingsState: "ok",
            settingsDetail: null,
            configDirWritable: true,
            shellEnv: [{ name: "ANTHROPIC_AUTH_TOKEN", length: 36 }],
          }),
      },
    });
    render(<App ipc={ipc} />);
    expect(
      await screen.findByText(/Your shell exports ANTHROPIC settings that override this app/),
    ).toBeInTheDocument();
    expect(screen.getByText(/ANTHROPIC_AUTH_TOKEN \(36 chars\)/)).toBeInTheDocument();
  });

  it("stays quiet when the shell exports nothing", async () => {
    const ipc = createMockIpc({
      handlers: {
        list_providers: () => Promise.resolve(providerSummaryFixtures),
        read_status: () => Promise.resolve(statusFixture),
        check_environment: () =>
          Promise.resolve({
            cliFound: true,
            cliVersion: "2.1.319",
            cliVersionWarning: null,
            settingsState: "ok",
            settingsDetail: null,
            configDirWritable: true,
            shellEnv: [],
          }),
      },
    });
    render(<App ipc={ipc} />);
    await screen.findByText(/direct \(credential helper\)/);
    expect(screen.queryByText(/Your shell exports ANTHROPIC settings/)).not.toBeInTheDocument();
  });

  describe('Test Connection (the only honest answer to "does my key work")', () => {
    function mockTesting(result: Record<string, unknown> | Error) {
      const calls: Array<Record<string, unknown>> = [];
      const ipc = createMockIpc({
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
          test_connection: (args) => {
            calls.push(args ?? {});
            return result instanceof Error ? Promise.reject(result) : Promise.resolve(result);
          },
        },
      });
      return { ipc, calls };
    }

    const okResult = {
      providerId: "openrouter",
      ok: true,
      status: 200,
      message: "key accepted by the provider",
      endpoint: "https://openrouter.ai/api/v1/key",
      latencyMs: 412,
    };

    it("is disabled until a provider and a key are present", async () => {
      const { ipc } = mockTesting(okResult);
      render(<App ipc={ipc} />);
      await goTo("Provider");
      expect(screen.getByRole("button", { name: "Test Connection" })).toBeDisabled();
      await selectProviderAndKey("openrouter");
      expect(screen.getByRole("button", { name: "Test Connection" })).toBeEnabled();
    });

    it("sends the typed key and reports a working connection", async () => {
      const { ipc, calls } = mockTesting(okResult);
      render(<App ipc={ipc} />);
      await selectProviderAndKey("openrouter");

      await userEvent.click(screen.getByRole("button", { name: "Test Connection" }));
      await waitFor(() => expect(calls).toHaveLength(1));
      expect(calls[0].providerId).toBe("openrouter");
      expect(calls[0].credential).toBe("sk-test-key");
      expect(
        await screen.findByText(/Connection OK — key accepted by the provider/),
      ).toBeInTheDocument();
    });

    it("reports a rejected key as a failure, not as success", async () => {
      const { ipc } = mockTesting({
        ...okResult,
        ok: false,
        status: 401,
        message:
          "the provider rejected this key (401 Unauthorized) — it is wrong, expired, or revoked",
      });
      render(<App ipc={ipc} />);
      await selectProviderAndKey("openrouter");

      await userEvent.click(screen.getByRole("button", { name: "Test Connection" }));
      expect(
        await screen.findByText(/Connection failed — the provider rejected this key/),
      ).toBeInTheDocument();
      // The success wording must not appear anywhere in the failure state.
      expect(screen.queryByText(/Connection OK/)).not.toBeInTheDocument();
    });

    it("surfaces a backend refusal (e.g. no documented endpoint) verbatim", async () => {
      const err = Object.assign(new Error("there is no documented key-check endpoint for Z.AI"), {
        name: "CommandError",
      });
      const { ipc } = mockTesting(err);
      render(<App ipc={ipc} />);
      await selectProviderAndKey("openrouter");

      await userEvent.click(screen.getByRole("button", { name: "Test Connection" }));
      expect(await screen.findByText(/Connection test could not run/)).toBeInTheDocument();
    });
  });

  async function selectProviderAndKey(provider = "anthropic") {
    await goTo("Provider");
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, provider);
    await userEvent.type(screen.getByLabelText("API key", { selector: "input" }), "sk-test-key");
  }

  it("sends the typed model id to save_and_apply", async () => {
    const { ipc, calls } = mockCapturing();
    render(<App ipc={ipc} />);
    await selectProviderAndKey();
    await goTo("Model");
    await userEvent.type(
      screen.getByLabelText("Model", { selector: "input" }),
      "claude-sonnet-4-5",
    );

    await userEvent.click(screen.getByRole("button", { name: "Save & Apply" }));
    await waitFor(() => expect(calls).toHaveLength(1));
    expect(calls[0].model).toBe("claude-sonnet-4-5");
    expect(calls[0].providerId).toBe("anthropic");
  });

  it("sends null when no model is chosen (never a guessed default)", async () => {
    const { ipc, calls } = mockCapturing();
    render(<App ipc={ipc} />);
    await selectProviderAndKey();

    await userEvent.click(screen.getByRole("button", { name: "Save & Apply" }));
    await waitFor(() => expect(calls).toHaveLength(1));
    expect(calls[0].model).toBeNull();
  });

  it("trims a padded model id before sending it", async () => {
    const { ipc, calls } = mockCapturing();
    render(<App ipc={ipc} />);
    await selectProviderAndKey();
    await goTo("Model");
    await userEvent.type(screen.getByLabelText("Model", { selector: "input" }), "  glm-4.6  ");

    await userEvent.click(screen.getByRole("button", { name: "Save & Apply" }));
    await waitFor(() => expect(calls).toHaveLength(1));
    expect(calls[0].model).toBe("glm-4.6");
  });

  it("blocks apply on an invalid model id and explains why (negative)", async () => {
    const { ipc, calls } = mockCapturing();
    render(<App ipc={ipc} />);
    await selectProviderAndKey();
    await goTo("Model");
    const modelInput = screen.getByLabelText("Model", { selector: "input" });
    await userEvent.type(modelInput, "bad model");

    expect(screen.getByRole("button", { name: "Save & Apply" })).toBeDisabled();
    expect(modelInput).toHaveAttribute("aria-invalid", "true");
    const error = screen.getByText(/printable ASCII/);
    expect(error).toBeInTheDocument();
    expect(error).toHaveAttribute("aria-live", "polite");
    expect(calls).toHaveLength(0);

    // Fixing the id re-enables apply.
    await userEvent.clear(modelInput);
    await userEvent.type(modelInput, "badmodel");
    expect(screen.getByRole("button", { name: "Save & Apply" })).toBeEnabled();
    expect(screen.queryByText(/printable ASCII/)).not.toBeInTheDocument();
  });

  it("applies a model chosen from a fetched catalog", async () => {
    const calls: Array<Record<string, unknown>> = [];
    const ipc = createMockIpc({
      handlers: {
        list_providers: () => Promise.resolve(providerSummaryFixtures),
        save_and_apply: (args) => {
          calls.push(args ?? {});
          return Promise.resolve(outcomeFixture);
        },
        discover_models: () =>
          Promise.resolve([
            {
              id: "anthropic/claude-x",
              displayName: "Claude X",
              contextWindow: null,
              capabilityStatus: "unknown",
              source: "https://openrouter.ai/api/v1/models",
            },
          ]),
      },
    });
    render(<App ipc={ipc} />);
    await goTo("Provider");
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "openrouter");
    await userEvent.type(screen.getByLabelText("API key", { selector: "input" }), "sk-test-key");
    await goTo("Model");
    await userEvent.click(screen.getByRole("button", { name: "Fetch models" }));
    await userEvent.click(await screen.findByRole("option", { name: /anthropic\/claude-x/ }));

    await userEvent.click(screen.getByRole("button", { name: "Save & Apply" }));
    await waitFor(() => expect(calls).toHaveLength(1));
    expect(calls[0].model).toBe("anthropic/claude-x");
    expect(calls[0].baseUrl).toBe("https://openrouter.ai/api");
  });
});

describe("Advanced role overrides + Save & Apply (T45)", () => {
  const outcomeFixture = {
    settingsPath: "/tmp/fixture/.claude/settings.json",
    backupFileName: "settings-backup-1700000000000000000.json",
    helperPath: "/tmp/fixture/helpers/anthropic.sh",
    appliedKeys: ["env", "apiKeyHelper"],
  };

  function mockCapturing() {
    const calls: Array<Record<string, unknown>> = [];
    const ipc = createMockIpc({
      handlers: {
        list_providers: () => Promise.resolve(providerSummaryFixtures),
        save_and_apply: (args) => {
          calls.push(args ?? {});
          return Promise.resolve(outcomeFixture);
        },
      },
    });
    return { ipc, calls };
  }

  async function ready() {
    await goTo("Provider");
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "anthropic");
    await userEvent.type(screen.getByLabelText("API key", { selector: "input" }), "sk-test-key");
  }

  it("keeps the panel collapsed and sends no role overrides by default", async () => {
    const { ipc, calls } = mockCapturing();
    render(<App ipc={ipc} />);
    await ready();
    await goTo("Model");

    expect(screen.getByRole("button", { name: /advanced settings/i })).toHaveAttribute(
      "aria-expanded",
      "false",
    );

    await userEvent.click(screen.getByRole("button", { name: "Save & Apply" }));
    await waitFor(() => expect(calls).toHaveLength(1));
    expect(calls[0].roleHaiku).toBeNull();
    expect(calls[0].roleSonnet).toBeNull();
    expect(calls[0].roleOpus).toBeNull();
  });

  it("sends only the role overrides the user filled in, trimmed", async () => {
    const { ipc, calls } = mockCapturing();
    render(<App ipc={ipc} />);
    await ready();
    await goTo("Model");
    await userEvent.click(screen.getByRole("button", { name: /advanced settings/i }));
    await userEvent.type(screen.getByLabelText("Haiku role"), "  fast-model  ");
    await userEvent.type(screen.getByLabelText("Opus role"), "big-model");

    await userEvent.click(screen.getByRole("button", { name: "Save & Apply" }));
    await waitFor(() => expect(calls).toHaveLength(1));
    expect(calls[0].roleHaiku).toBe("fast-model");
    expect(calls[0].roleSonnet).toBeNull();
    expect(calls[0].roleOpus).toBe("big-model");
  });

  it("blocks apply while a role override is invalid and recovers when fixed", async () => {
    const { ipc, calls } = mockCapturing();
    render(<App ipc={ipc} />);
    await ready();
    await goTo("Model");
    await userEvent.click(screen.getByRole("button", { name: /advanced settings/i }));

    const sonnet = screen.getByLabelText("Sonnet role");
    await userEvent.type(sonnet, "bad model");
    expect(sonnet).toHaveAttribute("aria-invalid", "true");
    const apply = screen.getByRole("button", { name: "Save & Apply" });
    expect(apply).toBeDisabled();
    expect(screen.getByText(/a role override is invalid/i)).toBeInTheDocument();

    await userEvent.clear(sonnet);
    await userEvent.type(sonnet, "good-model");
    expect(apply).toBeEnabled();

    await userEvent.click(apply);
    await waitFor(() => expect(calls).toHaveLength(1));
    expect(calls[0].roleSonnet).toBe("good-model");
  });

  it("shows the currently configured role value from read_status", async () => {
    const { ipc } = mockCapturing();
    render(<App ipc={ipc} />);
    await ready();
    await goTo("Model");
    await userEvent.click(screen.getByRole("button", { name: /advanced settings/i }));
    // Default fixture has no overrides → honest "no override" text, never blank.
    expect(
      screen.getAllByText(/no override \(Claude Code default\)/).length,
    ).toBeGreaterThanOrEqual(3);
  });
});

describe("Credentials panel (V2-21)", () => {
  const one = {
    id: "openrouter-1",
    providerId: "openrouter",
    label: "work",
    last4: "9z9z",
    priority: 0,
    enabled: true,
    legacy: false,
  };

  function mockWithCredential(list = [one]) {
    return createMockIpc({
      handlers: {
        list_providers: () => Promise.resolve(providerSummaryFixtures),
        read_status: () => Promise.resolve(statusFixture),
        check_environment: () => Promise.resolve(null),
        has_credential: () => Promise.resolve(true),
        list_credentials: () => Promise.resolve(list),
        add_credential: () =>
          Promise.resolve({ ...one, id: "openrouter-2", label: "second", last4: "abcd" }),
        save_and_apply: () =>
          Promise.resolve({
            settingsPath: "/Users/x/.claude/settings.json",
            backupFileName: "settings-backup-1.json",
            helperPath: "/Users/x/.claude-config-manager/helpers/openrouter.sh",
            appliedKeys: ["apiKeyHelper"],
          }),
      },
    });
  }

  it("shows stored keys masked and adds a new one", async () => {
    const ipc = mockWithCredential();
    render(<App ipc={ipc} />);
    await goTo("Provider");
    await userEvent.selectOptions(
      await screen.findByLabelText("Provider", { selector: "select" }),
      "openrouter",
    );
    await goTo("Credentials");
    expect(await screen.findByText("•••• 9z9z")).toBeInTheDocument();
    await userEvent.type(screen.getByLabelText(/label for the new key/i), "second");
    await userEvent.type(screen.getByLabelText(/new api key/i), "sk-test-fixture-2");
    await userEvent.click(screen.getByRole("button", { name: "Add" }));
    await waitFor(() => {
      expect(ipc.calls.find((c) => c.cmd === "add_credential")?.args).toMatchObject({
        providerId: "openrouter",
        label: "second",
        secret: "sk-test-fixture-2",
      });
    });
  });

  it("saves through the selected credential id", async () => {
    const ipc = mockWithCredential();
    render(<App ipc={ipc} />);
    await goTo("Provider");
    await userEvent.selectOptions(
      await screen.findByLabelText("Provider", { selector: "select" }),
      "openrouter",
    );
    await goTo("Credentials");
    await screen.findByText("•••• 9z9z");
    await userEvent.click(screen.getByRole("button", { name: "Save & Apply" }));
    await waitFor(() => expect(ipc.calls.some((c) => c.cmd === "save_and_apply")).toBe(true));
    expect(ipc.calls.find((c) => c.cmd === "save_and_apply")?.args).toMatchObject({
      credentialId: "openrouter-1",
    });
  });
});

describe("Context-window adoption (catalog-sourced)", () => {
  function mockWithWindows(models: ModelEntry[]) {
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
        discover_models: () => Promise.resolve(models),
      },
    });
  }

  const catalog: ModelEntry[] = [
    {
      id: "m-1m",
      displayName: null,
      contextWindow: 1_000_000,
      capabilityStatus: "unknown",
      source: "https://x",
    },
  ];

  it("offers the catalog-declared window and adopts it on click", async () => {
    const ipc = mockWithWindows(catalog);
    render(<App ipc={ipc} />);
    await userEvent.click(await screen.findByRole("button", { name: "Provider" }));
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await userEvent.selectOptions(select, "openrouter");
    await userEvent.click(await screen.findByRole("button", { name: "Model" }));
    await userEvent.click(await screen.findByRole("button", { name: "Fetch models" }));
    await userEvent.click(await screen.findByRole("option", { name: /m-1m/ }));
    expect(await screen.findByText(/Catalog declares/)).toBeInTheDocument();
    expect(screen.getByText("1,000,000")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "declare it" }));
    expect(screen.getByRole("button", { name: "declared" })).toBeDisabled();
  });
});
