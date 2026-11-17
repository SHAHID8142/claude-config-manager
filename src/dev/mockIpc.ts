// Browser-only fixture transport. No credentials or provider requests leave this process.
import type {
  CommandError,
  CredentialView,
  EffectiveStatus,
  IpcLike,
  RoutingConfig,
} from "../lib/ipc";
import { providerSummaryFixtures } from "../lib/fixtures/providers";

export function createDevMockIpc(): IpcLike {
  let sequence = 0;
  let running = false;
  let loginStart = false;
  let routing: RoutingConfig = { version: 1, enabled: false, targets: [] };
  // Explicit browser-only model editor fixture; no keys or provider requests.
  if (new URLSearchParams(window.location.search).has("modelOrder")) {
    routing.targets = [
      {
        id: "nvidia-group",
        providerId: "nvidia-nim",
        credentialId: null,
        model: "nvidia/nemotron-3-ultra-550b-a55b",
        fallbackModels: [],
        baseUrl: "https://integrate.api.nvidia.com",
        enabled: true,
      },
    ];
  }
  let credentials: CredentialView[] = [];
  let status: EffectiveStatus = {
    envDefaultHaiku: null,
    envDefaultSonnet: null,
    envDefaultOpus: null,
    envMaxContextTokens: null,
    settingsFound: false,
    model: null,
    envModel: null,
    envBaseUrl: null,
    authTokenPresent: false,
    apiKeyPresent: false,
    apiKeyHelper: null,
    warnings: [],
  };
  const failures = new URLSearchParams(window.location.search).get("fail");
  const clone = <T>(value: T): T => structuredClone(value);
  const outcome = () => ({
    settingsPath: "/Users/demo/.claude/settings.json",
    backupFileName: "fixture-backup.json",
    helperPath: status.apiKeyHelper,
    appliedKeys: ["env", "apiKeyHelper"],
  });
  const helper = () => ({
    running,
    stale: false,
    pid: running ? 48211 : null,
    port: running ? 31415 : null,
    loginStart,
    message: "Browser fixture; no helper process is started",
  });
  const handlers: Record<string, (a: Record<string, unknown>) => unknown> = {
    list_providers: () => providerSummaryFixtures,
    read_status: () => status,
    check_environment: () => ({
      cliFound: true,
      cliVersion: "fixture",
      cliVersionWarning: "Browser demonstration only",
      settingsState: "ok",
      settingsDetail: null,
      configDirWritable: true,
    }),
    config_paths: () => ({
      home: "/Users/demo",
      secretsDir: "/Users/demo/.claude-config-manager/secrets",
      helpersDir: "/Users/demo/.claude-config-manager/helpers",
      routingFile: "/Users/demo/.claude-config-manager/routing.json",
      settingsFile: "/Users/demo/.claude/settings.json",
      helperState: "/Users/demo/.claude-config-manager/proxy-helper.json",
    }),
    list_backups: () => [],
    backup_previews: () => [],
    switch_history: () => [],
    pending_transaction: () => null,
    recover_transaction: () => ({
      restoredPrevious: true,
      fileCount: 0,
      currentSnapshotFile: null,
    }),
    has_credential: (a) => credentials.some((k) => k.providerId === a.providerId && k.enabled),
    list_credentials: (a) =>
      credentials
        .filter((k) => k.providerId === a.providerId)
        .sort((a, b) => a.priority - b.priority),
    add_credential: (a) => {
      const key: CredentialView = {
        id: `fixture-${++sequence}`,
        providerId: String(a.providerId),
        label: String(a.label),
        last4: String(a.secret).slice(-4),
        priority: credentials.length,
        enabled: true,
        legacy: false,
      };
      credentials.push(key);
      return key;
    },
    remove_credential: (a) => {
      credentials = credentials.filter((k) => k.id !== a.credentialId);
    },
    set_credential_enabled: (a) => {
      const k = credentials.find((k) => k.id === a.credentialId);
      if (k) k.enabled = Boolean(a.enabled);
    },
    promote_credential: (a) => {
      const k = credentials.find((k) => k.id === a.credentialId);
      if (k) k.priority = Math.min(0, ...credentials.map((k) => k.priority)) - 1;
    },
    discover_models: () => [
      {
        id: "fixture-model",
        displayName: "Example model — fixture only",
        contextWindow: null,
        capabilityStatus: "unknown",
        source: "fixture",
      },
    ],
    test_connection: (a) => ({
      providerId: a.providerId,
      ok: true,
      status: 200,
      message: "Simulated response; no provider was contacted",
      endpoint: "fixture",
      latencyMs: 0,
    }),
    save_and_apply: (a) => {
      status = {
        ...status,
        settingsFound: true,
        model: (a.model as string) ?? status.model,
        envBaseUrl: (a.baseUrl as string) ?? null,
        apiKeyHelper: `/Users/demo/.claude-config-manager/helpers/${a.providerId}.sh`,
      };
      return outcome();
    },
    get_routing: () => routing,
    save_routing: (a) => {
      routing = clone(a.config as RoutingConfig);
    },
    proxy_status: () => ({
      running,
      port: running ? 31415 : null,
      requests: 0,
      fallbacks: 0,
      failures: 0,
      lastTarget: null,
      lastError: null,
      cooling: [],
      cooldownSeconds: {},
      lastSwitchReason: null,
      lastKeyLabel: null,
    }),
    proxy_start: () => {
      running = true;
      return { port: 31415, helperPath: "/fixture/helper" };
    },
    proxy_restart: () => {
      running = true;
      return { port: 31415, helperPath: "/fixture/helper" };
    },
    proxy_stop: () => {
      running = false;
      return true;
    },
    helper_status: helper,
    helper_login_start: (a) => {
      loginStart = Boolean(a.enabled);
      return helper();
    },
    check_routing_health: () =>
      routing.targets.map((t) => ({
        targetId: t.id,
        providerId: t.providerId,
        ok: true,
        message: "Simulated check",
      })),
    apply_routed_mode: (a) => {
      if (a.config) routing = structuredClone(a.config as RoutingConfig);
      running = true;
      status = {
        ...status,
        settingsFound: true,
        envBaseUrl: "http://127.0.0.1:31415",
        apiKeyHelper: "/Users/demo/.claude-config-manager/helpers/proxy.sh",
      };
      return outcome();
    },
    restore_last_backup: () => {
      throw { kind: "not_available", message: "No browser fixture backup exists" };
    },
    recover_settings: () => ({
      restoredFrom: "fixture",
      preRestoreBackup: "invalid-settings-fixture.json",
    }),
    bridge_overview: () => ({
      consented: false,
      installed: false,
      version: "3.1.3",
      nodeFound: false,
      nodeMajor: null,
      running: false,
      pid: null,
      gatewayPort: 3456,
    }),
  };
  const failMap: Record<string, string[]> = {
    catalog: ["list_providers"],
    env: ["check_environment", "read_status"],
    models: ["discover_models"],
    apply: ["save_and_apply", "apply_routed_mode"],
  };
  return {
    async invoke<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
      if (failMap[failures ?? ""]?.includes(cmd))
        throw { kind: "io", message: `Injected ${cmd} failure` } satisfies CommandError;
      if (!handlers[cmd])
        throw {
          kind: "not_available",
          message: `Unavailable in browser fixture: ${cmd}`,
        } satisfies CommandError;
      return clone(handlers[cmd](args)) as T;
    },
  };
}
