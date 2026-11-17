import { describe, expect, it } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import App from "./App";
import { createMockIpc } from "./lib/ipc-mock";
import { providerSummaryFixtures } from "./lib/fixtures/providers";
import type { CredentialView, EffectiveStatus, RoutingConfig } from "./lib/ipc";
import { CredentialsPanel } from "./components/CredentialsPanel";

const emptyStatus: EffectiveStatus = {
  settingsFound: true,
  model: null,
  envModel: null,
  envBaseUrl: null,
  apiKeyHelper: null,
  authTokenPresent: false,
  apiKeyPresent: false,
  envDefaultHaiku: null,
  envDefaultSonnet: null,
  envDefaultOpus: null,
  envMaxContextTokens: null,
  warnings: [],
};
function fixture(invalid = false) {
  const keys: Record<string, CredentialView[]> = {};
  let config: RoutingConfig = { version: 1, enabled: false, targets: [] };
  let broken = invalid;
  let routed = false;
  const ipc = createMockIpc({
    handlers: {
      list_providers: async () => providerSummaryFixtures,
      get_routing: async () => config,
      list_backups: async () => [],
      backup_previews: async () => [
        {
          fileName: "settings-backup-1.json",
          sizeBytes: 40,
          createdEpoch: 1,
          valid: true,
          detail: "Valid settings",
          model: "saved",
          baseUrl: null,
          helperPresent: false,
        },
      ],
      config_paths: async () => ({
        settingsFile: "/fixture/settings.json",
        secretsDir: "/fixture/secrets",
      }),
      helper_status: async () => ({
        running: routed,
        stale: false,
        pid: null,
        port: null,
        loginStart: false,
        message: routed ? "Running" : "Stopped",
      }),
      proxy_status: async () => null,
      read_status: async () => {
        if (broken) throw { kind: "io", message: "empty file, 0 bytes" };
        return { ...emptyStatus, envBaseUrl: routed ? "http://127.0.0.1:31415" : null };
      },
      check_environment: async () => ({
        cliFound: true,
        cliVersion: "2.1.325",
        cliVersionWarning: null,
        settingsState: broken ? "malformed_json" : "ok",
        settingsDetail: broken ? "empty file, 0 bytes" : null,
        configDirWritable: true,
      }),
      list_credentials: async (args) => keys[String(args?.providerId)] ?? [],
      add_credential: async (args) => {
        const providerId = String(args?.providerId);
        const list = (keys[providerId] ??= []);
        const key = {
          id: `${providerId}-${list.length + 1}`,
          providerId,
          label: String(args?.label),
          last4: String(args?.secret).slice(-4),
          enabled: true,
          priority: list.length,
          legacy: false,
        };
        list.push(key);
        return key;
      },
      save_and_apply: async () => ({
        settingsPath: "/fixture/settings.json",
        helperPath: "/fixture/anthropic.sh",
        backupFileName: "snapshot.json",
        appliedKeys: ["model", "apiKeyHelper"],
      }),
      save_routing: async (args) => {
        config = structuredClone(args?.config as RoutingConfig);
      },
      apply_routed_mode: async (args) => {
        if (args?.config) config = structuredClone(args.config as RoutingConfig);
        routed = true;
        return {
          settingsPath: "/fixture/settings.json",
          helperPath: "/fixture/proxy.sh",
          backupFileName: "snapshot.json",
          appliedKeys: ["env"],
        };
      },
      recover_settings: async () => {
        broken = false;
        return {
          restoredFrom: "fixture-backup.json",
          preRestoreBackup: "invalid-settings-fixture.json",
        };
      },
    },
  });
  return { ipc, keys, getConfig: () => config };
}

describe("repaired user workflow", () => {
  it("adds provider-owned keys and model mappings, then saves one provider-first switching policy", async () => {
    const user = userEvent.setup();
    const f = fixture();
    render(<App ipc={f.ipc} />);
    for (const [provider, model, label] of [
      ["openrouter", "a-model", "Work"],
      ["deepseek", "b-model", "Backup"],
    ]) {
      await user.click(screen.getByRole("button", { name: "Providers & keys" }));
      await user.selectOptions(
        await screen.findByLabelText("Provider", { selector: "select" }),
        provider,
      );
      await user.type(await screen.findByLabelText("Key label", { selector: "input" }), label);
      await user.type(
        screen.getByLabelText("New API key", { selector: "input" }),
        "fixture-only-1234",
      );
      await user.click(screen.getByRole("button", { name: "Add key" }));
      await waitFor(() => expect(f.keys[provider]).toHaveLength(1));
      await user.type(screen.getByLabelText("Model", { selector: "input" }), model);
      await user.click(screen.getByRole("button", { name: "Add provider to switching order" }));
      expect(
        await screen.findByRole("button", { name: "Save & enable automatic switching" }),
      ).toBeInTheDocument();
    }
    await user.click(screen.getByRole("button", { name: "Save & enable automatic switching" }));
    await waitFor(() =>
      expect(f.ipc.calls.some((call) => call.cmd === "apply_routed_mode")).toBe(true),
    );
    expect(f.getConfig().enabled).toBe(true);
    expect(
      f.getConfig().targets.map((target) => [target.providerId, target.credentialId, target.model]),
    ).toEqual([
      ["openrouter", null, "a-model"],
      ["deepseek", null, "b-model"],
    ]);
    expect(JSON.stringify(f.getConfig())).not.toContain("fixture-only");
    expect(f.ipc.calls.find((call) => call.cmd === "apply_routed_mode")?.args?.config).toEqual(
      f.getConfig(),
    );
    expect(f.ipc.calls.some((call) => call.cmd === "save_routing")).toBe(false);
  });

  it("enables NVIDIA Save with key/model and keeps supported provider Save working", async () => {
    const user = userEvent.setup();
    const f = fixture();
    f.keys["nvidia-nim"] = [
      {
        id: "nim-fixture",
        providerId: "nvidia-nim",
        label: "Work",
        last4: "1234",
        priority: 0,
        enabled: true,
        legacy: false,
      },
    ];
    f.keys.anthropic = [
      { ...f.keys["nvidia-nim"][0], id: "anthropic-fixture", providerId: "anthropic" },
    ];
    render(<App ipc={f.ipc} />);
    await user.click(screen.getByRole("button", { name: "Providers & keys" }));
    const provider = await screen.findByLabelText("Provider", { selector: "select" });
    await user.selectOptions(provider, "nvidia-nim");
    await user.type(
      screen.getByLabelText("Model", { selector: "input" }),
      "nvidia/nemotron-3-ultra-550b-a55b",
    );
    expect(
      await screen.findByText("NVIDIA uses the built-in local connection."),
    ).toBeInTheDocument();
    expect(screen.queryByText(/capabilities verified/)).not.toBeInTheDocument();
    const apply = screen.getByRole("button", { name: "Save & Apply" });
    await waitFor(() => expect(apply).toBeEnabled());
    expect(apply).toHaveAccessibleDescription(
      "Save activates NVIDIA through the local helper and puts its keys first in switching order.",
    );
    await user.click(apply);
    await waitFor(() =>
      expect(
        f.ipc.calls.some((c) => c.cmd === "save_and_apply" && c.args?.providerId === "nvidia-nim"),
      ).toBe(true),
    );
    await user.click(screen.getByText("Single-key direct connection (optional)"));
    expect(screen.getByRole("button", { name: "Test Connection" })).toBeDisabled();
    await user.selectOptions(provider, "anthropic");
    await user.type(screen.getByLabelText("Model", { selector: "input" }), "claude-sonnet-4-5");
    await waitFor(() => expect(screen.getByRole("button", { name: "Save & Apply" })).toBeEnabled());
    await user.click(screen.getByRole("button", { name: "Save & Apply" }));
    await waitFor(() => expect(f.ipc.calls.some((c) => c.cmd === "save_and_apply")).toBe(true));
    expect(await screen.findByText("Configuration saved and verified.")).toBeInTheDocument();
    expect(f.keys["nvidia-nim"]).toHaveLength(1);
  });

  it("explains the missing NVIDIA key and enables Save only after the key is stored", async () => {
    const user = userEvent.setup();
    const f = fixture();
    render(<App ipc={f.ipc} />);
    await user.click(screen.getByRole("button", { name: "Providers & keys" }));
    await user.selectOptions(
      await screen.findByLabelText("Provider", { selector: "select" }),
      "nvidia-nim",
    );
    await user.type(
      screen.getByLabelText("Model", { selector: "input" }),
      "nvidia/nemotron-3-ultra-550b-a55b",
    );
    expect(screen.getByRole("button", { name: "Save & Apply" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Save & Apply" })).toHaveAccessibleDescription(
      "Add a NVIDIA key to enable Save & Apply.",
    );
    await user.type(screen.getByLabelText("Key label", { selector: "input" }), "Work");
    await user.type(screen.getByLabelText("New API key", { selector: "input" }), "fixture-key");
    await user.click(screen.getByRole("button", { name: "Add key" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Save & Apply" })).toBeEnabled());
  });

  it("requires explicit recovery, then clears invalid-file state after success", async () => {
    const user = userEvent.setup();
    const f = fixture(true);
    render(<App ipc={f.ipc} />);
    await user.click(await screen.findByRole("button", { name: "Restore valid backup…" }));
    expect(f.ipc.calls.some((call) => call.cmd === "recover_settings")).toBe(false);
    await user.click(screen.getByRole("button", { name: "Preserve original & recover" }));
    await waitFor(() =>
      expect(screen.queryByRole("region", { name: "Settings recovery" })).not.toBeInTheDocument(),
    );
    expect(f.ipc.calls.find((call) => call.cmd === "recover_settings")?.args).toEqual({
      useBackup: true,
      fileName: "settings-backup-1.json",
    });
  });

  it("confirms interrupted-save recovery before replacing managed files", async () => {
    const user = userEvent.setup();
    let pending = true;
    const ipc = createMockIpc({
      handlers: {
        list_providers: async () => providerSummaryFixtures,
        read_status: async () => emptyStatus,
        pending_transaction: async () =>
          pending
            ? { createdEpoch: 1, fileCount: 4, settingsPath: "/fixture/settings.json" }
            : null,
        recover_transaction: async () => {
          pending = false;
          return {
            restoredPrevious: true,
            fileCount: 4,
            currentSnapshotFile: "recovery-before-fixture.json",
          };
        },
      },
    });
    render(<App ipc={ipc} />);
    await user.click(
      await screen.findByRole("button", { name: "Restore previous managed state…" }),
    );
    expect(ipc.calls.some((c) => c.cmd === "recover_transaction")).toBe(false);
    await user.click(screen.getByRole("button", { name: "Confirm recovery choice" }));
    await waitFor(() =>
      expect(
        screen.queryByRole("region", { name: "Interrupted save recovery" }),
      ).not.toBeInTheDocument(),
    );
    expect(ipc.calls.find((c) => c.cmd === "recover_transaction")?.args).toEqual({
      restorePrevious: true,
    });
  });

  it("keeps provider model drafts across switching and relaunch without storing typed keys", async () => {
    const user = userEvent.setup();
    const f = fixture();
    const first = render(<App ipc={f.ipc} />);
    await user.click(screen.getByRole("button", { name: "Providers & keys" }));
    const select = await screen.findByLabelText("Provider", { selector: "select" });
    await user.selectOptions(select, "openrouter");
    await user.type(screen.getByLabelText("Model", { selector: "input" }), "openrouter-draft");
    await user.click(screen.getByText("Single-key direct connection (optional)"));
    await user.type(
      screen.getByLabelText("API key", { selector: "input" }),
      "fixture-private-never-persist",
    );
    await user.selectOptions(select, "deepseek");
    expect(screen.getByLabelText("Model", { selector: "input" })).toHaveValue("");
    await user.type(screen.getByLabelText("Model", { selector: "input" }), "deepseek-draft");
    await user.selectOptions(select, "openrouter");
    expect(screen.getByLabelText("Model", { selector: "input" })).toHaveValue("openrouter-draft");
    expect(screen.getByLabelText("API key", { selector: "input" })).toHaveValue("");
    const raw = window.localStorage.getItem("ccm.session.provider-drafts-v1");
    expect(raw).toContain("openrouter-draft");
    expect(raw).not.toContain("fixture-private-never-persist");
    first.unmount();
    render(<App ipc={f.ipc} />);
    expect(await screen.findByLabelText("Model", { selector: "input" })).toHaveValue(
      "openrouter-draft",
    );
    expect(screen.getByRole("region", { name: "Review changes" })).toHaveTextContent(
      "Pending model",
    );
  });

  it("warns about a legacy direct NVIDIA helper instead of implying the connection is ready", async () => {
    const ipc = createMockIpc({
      handlers: {
        list_providers: async () => providerSummaryFixtures,
        read_status: async () => ({
          ...emptyStatus,
          apiKeyHelper: "'/Users/fixture space/.claude-config-manager/helpers/nvidia-nim.sh'",
        }),
      },
    });
    render(<App ipc={ipc} />);
    expect(await screen.findByText("Saved NVIDIA setup needs activation.")).toBeInTheDocument();
    expect(screen.getByText(/without the local translation endpoint/)).toBeInTheDocument();
    expect(ipc.calls.some((c) => c.cmd === "save_and_apply")).toBe(false);
  });

  it("keeps a failed key submission for retry", async () => {
    const user = userEvent.setup();
    render(
      <CredentialsPanel
        credentials={[]}
        onAdd={async () => false}
        onRemove={() => {}}
        onToggle={() => {}}
        onPromote={() => {}}
        error="Cannot save"
      />,
    );
    await user.type(screen.getByLabelText("Key label", { selector: "input" }), "Work");
    await user.type(screen.getByLabelText("New API key", { selector: "input" }), "fixture-key");
    await user.click(screen.getByRole("button", { name: "Add key" }));
    expect(screen.getByLabelText("New API key", { selector: "input" })).toHaveValue("fixture-key");
    expect(screen.getByRole("alert")).toHaveTextContent("Cannot save");
  });
});
