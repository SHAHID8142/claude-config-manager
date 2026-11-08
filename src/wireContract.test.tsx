import { describe, expect, it } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import App from "./App";
import { createMockIpc } from "./lib/ipc-mock";
import type { EffectiveStatus, ModelEntry } from "./lib/ipc";

// Payloads captured verbatim from the Rust DTOs (serde output), not hand-written.
const REAL_PROVIDERS = [
  {
    id: "anthropic",
    name: "Anthropic",
    protocol: "anthropic_native",
    bridge: "not_required",
    capabilityStatus: {
      status: "verified",
      source_url: "https://platform.claude.com/docs/en/api/models-list",
    },
    docsUrl: "https://platform.claude.com/docs/en/api/models-list",
    directBaseUrl: null,
  },
  {
    id: "openrouter",
    name: "OpenRouter",
    protocol: "anthropic_compat",
    bridge: "not_required",
    capabilityStatus: {
      status: "verified",
      source_url: "https://openrouter.ai/docs/api-reference/messages",
    },
    docsUrl: "https://openrouter.ai/docs/cookbook/coding-agents/claude-code-integration",
    directBaseUrl: "https://openrouter.ai/api",
  },
  {
    id: "deepseek",
    name: "DeepSeek",
    protocol: "anthropic_compat",
    bridge: "not_required",
    capabilityStatus: {
      status: "verified",
      source_url: "https://api-docs.deepseek.com/guides/anthropic_api",
    },
    docsUrl: "https://api-docs.deepseek.com/guides/anthropic_api",
    directBaseUrl: "https://api.deepseek.com/anthropic",
  },
  {
    id: "zai",
    name: "Z.AI",
    protocol: "anthropic_compat",
    bridge: "not_required",
    capabilityStatus: { status: "verified", source_url: "https://docs.z.ai/devpack/tool/claude" },
    docsUrl: "https://docs.z.ai/devpack/tool/claude",
    directBaseUrl: "https://api.z.ai/api/anthropic",
  },
  {
    id: "nvidia-nim",
    name: "NVIDIA NIM",
    protocol: "open_ai_only",
    bridge: "required_for_open_ai_only",
    capabilityStatus: {
      status: "verified",
      source_url: "https://docs.nvidia.com/nim/large-language-models/latest/introduction.html",
    },
    docsUrl: "https://docs.nvidia.com/nim/large-language-models/latest/introduction.html",
    directBaseUrl: null,
  },
] as never;
const REAL_STATUS = {
  settingsFound: true,
  model: "auto",
  envModel: "deepseek-chat",
  envBaseUrl: "http://127.0.0.1:31415",
  authTokenPresent: false,
  apiKeyPresent: true,
  apiKeyHelper: "/tmp/helpers/deepseek.sh",
  envDefaultHaiku: null,
  envDefaultSonnet: null,
  envDefaultOpus: null,
  warnings: [
    "helperShadowedByEnv",
    { plaintextKeyInEnv: { which: "ANTHROPIC_API_KEY" } },
    { modelConflict: { settingModel: "auto", envModel: "deepseek-chat" } },
  ],
} as unknown as EffectiveStatus;
const REAL_ENV = {
  cliFound: true,
  cliVersion: "2.1.316",
  cliVersionWarning: null,
  settingsState: "ok",
  settingsDetail: null,
  configDirWritable: true,
} as never;

function realIpc(overrides: Record<string, unknown> = {}) {
  return createMockIpc({
    handlers: {
      list_providers: () => Promise.resolve(REAL_PROVIDERS as never),
      read_status: () => Promise.resolve(REAL_STATUS),
      check_environment: () => Promise.resolve(REAL_ENV as never),
      ...overrides,
    },
  });
}

describe("real backend payloads (contract reproduction)", () => {
  it("renders without throwing and names the conflicting models", async () => {
    render(<App ipc={realIpc()} />);
    // The bug seen in the running app: literal \"undefined\" in the warning.
    const warning = await screen.findByText(/Model conflict/);
    expect(warning.textContent).toContain("auto");
    expect(warning.textContent).toContain("deepseek-chat");
    expect(warning.textContent).not.toContain("undefined");
  });

  it("survives a successful save followed by a status refresh (black-screen repro)", async () => {
    const user = userEvent.setup();
    const ipc = realIpc({
      save_and_apply: () =>
        Promise.resolve({
          settingsPath: "/Users/x/.claude/settings.json",
          backupFileName: "settings.json.2026-11-08T12-00-00Z.bak",
          helperPath:
            "/Users/x/Library/Application Support/ClaudeConfigManager/helpers/deepseek.sh",
          appliedKeys: ["model", "env.ANTHROPIC_BASE_URL", "apiKeyHelper"],
        }),
      discover_models: () =>
        Promise.resolve([
          {
            id: "deepseek-chat",
            displayName: "DeepSeek Chat",
            capabilityStatus: "unknown",
            source: "https://api.deepseek.com/models",
          },
        ] as ModelEntry[]),
    });
    render(<App ipc={ipc} />);
    await screen.findByRole("combobox", { name: /Provider/i });
    await user.selectOptions(screen.getByRole("combobox", { name: /Provider/i }), "deepseek");
    await user.type(
      screen.getByLabelText("API key", { selector: "input" }),
      "sk-test-key-1234567890",
    );
    const applyButton = screen.getByRole("button", { name: "Save & Apply" });
    await waitFor(() => expect(applyButton).toBeEnabled());
    await user.click(applyButton);
    // The app must still be interactive after an apply, not a blank tree.
    await waitFor(() =>
      expect(screen.getByRole("heading", { name: "Claude Config Manager" })).toBeInTheDocument(),
    );
  });
});
