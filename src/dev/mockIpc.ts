// Dev-only mock transport (never bundled in production: gated behind
// import.meta.env.DEV in main.tsx). Renders the full app in a plain browser
// for visual/UX audits.
import type { CommandError, IpcLike } from "../lib/ipc";
import { providerSummaryFixtures } from "../lib/fixtures/providers";

const err = (kind: CommandError["kind"], message: string) => Promise.reject({ kind, message });

const credential = {
  id: "cred-1",
  providerId: "anthropic",
  label: "Personal key",
  last4: "9xQf",
  priority: 1,
  enabled: true,
  legacy: false,
};

const models = [
  "claude-sonnet-4-5",
  "claude-opus-4-1",
  "claude-haiku-4-0",
  "deepseek/deepseek-v4-pro",
  "openai/gpt-5.2",
  "google/gemini-3-pro",
  "meta-llama/llama-4-405b",
  "mistralai/mistral-large-3",
  "qwen/qwen3-max",
  "x-ai/grok-4",
].map((id) => ({
  id,
  displayName: null,
  capabilityStatus: "unknown" as const,
  source: "https://openrouter.ai/api/v1/models",
}));

/** Dev-only failure injection: `?fail=catalog|env|models|apply|bridge`
 * makes the named domain reject, so error states can be verified visually. */
const failParam = new URLSearchParams(window.location.search).get("fail") ?? "";

export function createDevMockIpc(): IpcLike {
  // Stateful across calls so consent/install/start flows can be walked.
  let consented = false;
  let installed = false;
  let running = false;
  const handlers: Record<string, (args?: Record<string, unknown>) => Promise<unknown>> = {
    list_providers: () => Promise.resolve(providerSummaryFixtures),
    read_status: () =>
      Promise.resolve({
        settingsFound: true,
        model: "claude-sonnet-4-5",
        envModel: null,
        envBaseUrl: "https://api.anthropic.com",
        authTokenPresent: false,
        apiKeyPresent: false,
        apiKeyHelper:
          "/Users/demo/Library/Application Support/ClaudeConfigManager/helpers/anthropic.sh",
        warnings: [],
      }),
    check_environment: () =>
      Promise.resolve({
        cliFound: true,
        cliVersion: "2.1.319",
        cliVersionWarning: null,
        settingsState: "ok",
        settingsDetail: null,
        configDirWritable: true,
      }),
    has_credential: () => Promise.resolve(true),
    list_credentials: () => Promise.resolve([credential]),
    add_credential: (a) =>
      Promise.resolve({
        ...credential,
        id: `cred-${Date.now()}`,
        label: String(a?.label ?? "New key"),
        last4: String(a?.secret ?? "").slice(-4),
      }),
    remove_credential: () => Promise.resolve(true),
    set_credential_enabled: () => Promise.resolve(true),
    promote_credential: () => Promise.resolve(true),
    discover_models: () => Promise.resolve(models),
    test_connection: () =>
      Promise.resolve({
        providerId: "anthropic",
        ok: true,
        status: 200,
        message: "key accepted",
        endpoint: "https://api.anthropic.com",
        latencyMs: 214,
      }),
    save_and_apply: () =>
      Promise.resolve({
        settingsPath: "/Users/demo/.claude/settings.json",
        backupFileName: "settings-backup-1731500000000000000.json",
        helperPath:
          "/Users/demo/Library/Application Support/ClaudeConfigManager/helpers/anthropic.sh",
        appliedKeys: ["model", "apiKeyHelper"],
      }),
    restore_last_backup: () =>
      Promise.resolve({
        restoredFrom: "settings-backup-1731400000000000000.json",
        preRestoreBackup: "settings-backup-1731450000000000000.json",
      }),
    config_paths: () =>
      Promise.resolve({
        home: "/Users/demo",
        secretsDir: "/Users/demo/Library/Application Support/ClaudeConfigManager/secrets",
        helpersDir: "/Users/demo/Library/Application Support/ClaudeConfigManager/helpers",
        routingFile: "/Users/demo/Library/Application Support/ClaudeConfigManager/routing.json",
        settingsFile: "/Users/demo/.claude/settings.json",
        helperState:
          "/Users/demo/Library/Application Support/ClaudeConfigManager/helper-state.json",
      }),
    get_routing: () =>
      Promise.resolve({
        version: 3,
        enabled: true,
        targets: [
          {
            id: "t1",
            providerId: "anthropic",
            credentialId: "cred-1",
            model: "claude-sonnet-4-5",
            baseUrl: "https://api.anthropic.com",
            enabled: true,
          },
          {
            id: "t2",
            providerId: "openrouter",
            credentialId: null,
            model: "deepseek/deepseek-v4-pro",
            baseUrl: "https://openrouter.ai/api",
            enabled: true,
          },
        ],
      }),
    save_routing: () => Promise.resolve(undefined),
    proxy_status: () =>
      Promise.resolve({
        running: true,
        port: 31415,
        requests: 42,
        fallbacks: 2,
        failures: 0,
        lastTarget: "t1",
        pid: 48211,
        lastError: null,
        cooling: [],
      }),
    proxy_start: () => Promise.resolve({ port: 31415, helperPath: "/usr/local/bin/ccm-helper" }),
    proxy_stop: () => Promise.resolve(true),
    proxy_restart: () => Promise.resolve({ port: 31415, helperPath: "/usr/local/bin/ccm-helper" }),
    helper_status: () =>
      Promise.resolve({
        running: true,
        stale: false,
        pid: 48211,
        port: 31415,
        loginStart: true,
        message: "detached helper running",
      }),
    helper_login_start: () =>
      Promise.resolve({
        running: true,
        stale: false,
        pid: 48211,
        port: 31415,
        loginStart: true,
        message: "LaunchAgent installed",
      }),
    check_routing_health: () =>
      Promise.resolve([
        { targetId: "t1", providerId: "anthropic", ok: true, message: "key accepted (200)" },
        { targetId: "t2", providerId: "openrouter", ok: false, message: "401 unauthorized" },
      ]),
    apply_routed_mode: () =>
      Promise.resolve({
        settingsPath: "/Users/demo/.claude/settings.json",
        backupFileName: "settings-backup-1731500000000000001.json",
        helperPath:
          "/Users/demo/Library/Application Support/ClaudeConfigManager/helpers/routing.sh",
        appliedKeys: ["env", "apiKeyHelper"],
      }),
    bridge_consent_info: () =>
      Promise.resolve({
        package: "@musistudio/claude-code-router",
        version: "3.1.3",
        tarballUrl:
          "https://registry.npmjs.org/@musistudio/claude-code-router/-/claude-code-router-3.1.3.tgz",
        integritySha512B64:
          "sha512-dVHRu9Sb/xG9dgr8nHJmvd5XVIQ5yrEeKmMoPUEBc9xtgykfRT2z76NRI+fylpeiMzYC60z6vrV6x/3IY52pvg==",
        minNodeMajor: 22,
        license: "MIT",
        licenseSource: "https://github.com/musistudio/claude-code-router",
        tarballBytes: 6529968,
      }),
    bridge_overview: () =>
      Promise.resolve({
        consented,
        installed,
        version: "3.1.3",
        nodeFound: true,
        nodeMajor: 26,
        running,
        pid: running ? 48212 : null,
        gatewayPort: 3456,
      }),
    bridge_consent_record: () => {
      consented = true;
      return Promise.resolve(undefined);
    },
    bridge_consent_withdraw: () => {
      consented = false;
      running = false;
      installed = false;
      return Promise.resolve(undefined);
    },
    bridge_install: () => {
      installed = true;
      return Promise.resolve({ version: "3.1.3", installDir: "/tmp/mock", integrityVerified: true });
    },
    bridge_start: () => {
      running = true;
      return Promise.resolve(48212);
    },
    bridge_stop: () => {
      running = false;
      return Promise.resolve(true);
    },
    bridge_uninstall: () => Promise.resolve(undefined),
    bridge_apply_mode: () =>
      Promise.resolve({
        settingsPath: "/Users/demo/.claude/settings.json",
        backupFileName: "settings-backup-1731500000000000002.json",
        helperPath:
          "/Users/demo/Library/Application Support/ClaudeConfigManager/helpers/routing.sh",
        appliedKeys: ["env"],
      }),
  };

  const failMap: Record<string, string[]> = {
    catalog: ["list_providers"],
    env: ["check_environment", "read_status"],
    models: ["discover_models"],
    apply: ["save_and_apply", "apply_routed_mode"],
    bridge: ["bridge_overview", "bridge_install", "bridge_start"],
  };
  const failing = new Set(failMap[failParam] ?? []);

  return {
    invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
      if (failing.has(cmd)) {
        return err("io", `injected failure for visual QA ('${cmd}')`) as Promise<T>;
      }
      const h = handlers[cmd];
      if (!h) return err("not_available", `no dev-mock handler for '${cmd}'`) as Promise<T>;
      return h(args).then((v) => v as T);
    },
  };
}
