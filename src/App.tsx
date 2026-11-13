import { useCallback, useEffect, useState } from "react";
import { LoadingButton } from "./components/LoadingButton";
import {
  EMPTY_ROLES,
  hasInvalidRole,
  rolesToSend,
  type RoleState,
} from "./lib/roles";
import { AppHeader } from "./components/AppHeader";
import { AppSidebar } from "./components/AppSidebar";
import { ThemeSegmented } from "./components/ThemeSegmented";
import { DashboardTab } from "./tabs/DashboardTab";
import { ProviderTab } from "./tabs/ProviderTab";
import { CredentialsTab } from "./tabs/CredentialsTab";
import { ModelTab } from "./tabs/ModelTab";
import { RoutingTab } from "./tabs/RoutingTab";
import { MaintenanceTab } from "./tabs/MaintenanceTab";
import { TAB_META, isTabId, type TabId } from "./lib/tab";
import { loadSessionString, saveSessionString, clearSessionString } from "./lib/session";
import { StatusBanner } from "./components/StatusBanner";
import {
  checkEnvironment,
  defaultIpc,
  listBackups,
  discoverModels,
  testConnection,
  hasCredential,
  addCredential,
  listCredentials,
  promoteCredential,
  removeCredential,
  setCredentialEnabled,
  listProviders,
  readStatus,
  restoreLastBackup,
  saveAndApply,
  isCommandError,
  getRouting,
  saveRouting,
  proxyStart,
  proxyStop,
  proxyStatus,
  proxyRestart,
  helperStatus,
  helperLoginStart,
  configPaths,
  applyRoutedMode,
  checkRoutingHealth,
  type ApplyOutcome,
  type BackupInfo,
  type ConnectionTestResult,
  type CredentialView,
  type EffectiveStatus,
  type EnvironmentReport,
  type IpcLike,
  type ModelEntry,
  type ProviderSummary,
  type RestoreOutcome,
  type ConfigPaths,
  type HelperStatus,
  type ProxyStatus,
  type RoutingConfig,
  type TargetHealth,
} from "./lib/ipc";
import { validateModelId } from "./lib/modelId";
import { isLoopback } from "./lib/connectionLabel";

/** Sentinels for the provider selector: presets use their real ids;
 * custom entry and the empty state use dedicated sentinels. */
const CUSTOM_ID = "__custom__";
const EMPTY_ID = "__none__";

export interface AppProps {
  /** Transport seam for tests; production uses the Tauri runtime. */
  ipc?: IpcLike;
}

/**
 * Minimal single-window layout (T12) with a working provider selector and
 * custom-provider entry (T18). Controls not yet wired render disabled with
 * honest placeholders — never fabricated status.
 */
export default function App({ ipc = defaultIpc }: AppProps) {
  const [providers, setProviders] = useState<ProviderSummary[] | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  // D1: the working provider and active screen survive relaunch (macOS
  // "restore the last-displayed panel"); the stored id is validated once the
  // provider catalog arrives, so a removed provider degrades to none.
  const [selectedId, setSelectedId] = useState<string>(
    () => loadSessionString("provider") ?? EMPTY_ID,
  );
  const [customName, setCustomName] = useState("");
  const [customEndpoint, setCustomEndpoint] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [applying, setApplying] = useState(false);
  const [applyOutcome, setApplyOutcome] = useState<ApplyOutcome | null>(null);
  const [applyError, setApplyError] = useState<string | null>(null);
  const [status, setStatus] = useState<EffectiveStatus | null>(null);
  const [restoring, setRestoring] = useState(false);
  const [restoreOutcome, setRestoreOutcome] = useState<RestoreOutcome | null>(null);
  const [restoreError, setRestoreError] = useState<string | null>(null);
  const [environment, setEnvironment] = useState<EnvironmentReport | null>(null);
  const [backups, setBackups] = useState<BackupInfo[] | null>(null);
  const [shellWarningDismissed, setShellWarningDismissed] = useState(false);
  const [models, setModels] = useState<ModelEntry[] | null>(null);
  const [fetchingModels, setFetchingModels] = useState(false);
  const [modelsError, setModelsError] = useState<string | null>(null);
  const [modelsFetchedFor, setModelsFetchedFor] = useState<string | null>(null);
  const [testingConnection, setTestingConnection] = useState(false);
  const [connectionTest, setConnectionTest] = useState<ConnectionTestResult | null>(null);
  const [connectionTestError, setConnectionTestError] = useState<string | null>(null);
  const [selectedModel, setSelectedModel] = useState<string | null>(null);
  // T45: role overrides start empty = "no override" and stay collapsed.
  const [roles, setRoles] = useState<RoleState>(EMPTY_ROLES);
  const [advancedOpen, setAdvancedOpen] = useState(false);
  // One screen at a time: the sidebar swaps the panel, never adds to it.
  // D1: the last screen is restored on relaunch.
  const [tab, setTab] = useState<TabId>(() => {
    const saved = loadSessionString("tab");
    return isTabId(saved ?? "") ? (saved as TabId) : "dashboard";
  });
  // D1: every navigation is remembered for the next launch.
  const navigate = useCallback((next: TabId) => {
    setTab(next);
    saveSessionString("tab", next);
  }, []);
  // V2-12b: declared window for models Claude Code can't recognize.
  const [contextWindow, setContextWindow] = useState("");

  // V2-12a: a stored credential means the key field can stay blank.
  const [credentialCheck, setCredentialCheck] = useState<{
    providerId: string;
    has: boolean;
  } | null>(null);
  // V2-21: masked credential list for the selected provider.
  const [credentials, setCredentials] = useState<{
    providerId: string;
    list: CredentialView[];
  } | null>(null);
  const [credentialsError, setCredentialsError] = useState<string | null>(null);
  // V2-23/30: failover chain and proxy state, independent of the provider pick.
  const [routing, setRouting] = useState<RoutingConfig | null>(null);
  const [routingError, setRoutingError] = useState<string | null>(null);
  const [proxy, setProxy] = useState<ProxyStatus | null>(null);
  const [routingBusy, setRoutingBusy] = useState(false);
  const [helper, setHelper] = useState<HelperStatus | null>(null);
  const [paths, setPaths] = useState<ConfigPaths | null>(null);
  // V3-25: switching away from routed mode needs a second, deliberate click.
  const [confirmingDirect, setConfirmingDirect] = useState(false);
  // V3-27: the routing screen keeps its own list, so picking a provider there
  // can never overwrite what the Provider/Credentials screens are showing.
  const [routingDraftProviderId, setRoutingDraftProviderId] = useState<string | null>(null);
  const [routingCredentials, setRoutingCredentials] = useState<{
    providerId: string;
    list: CredentialView[];
  } | null>(null);
  const [routingHealth, setRoutingHealth] = useState<TargetHealth[] | null>(null);
  const [checkingHealth, setCheckingHealth] = useState(false);

  useEffect(() => {
    let cancelled = false;
    getRouting(ipc)
      .then((cfg) => {
        if (!cancelled) setRouting(cfg);
      })
      .catch((err: unknown) => {
        if (!cancelled) {
          setRoutingError(
            isCommandError(err) ? err.message : "The routing configuration could not be read.",
          );
        }
      });
    return () => {
      cancelled = true;
    };
  }, [ipc]);

  useEffect(() => {
    let cancelled = false;
    proxyStatus(ipc)
      .then((st) => {
        if (!cancelled) setProxy(st);
      })
      .catch(() => {
        /* The proxy may never have been started; that is not an error. */
      });
    helperStatus(ipc)
      .then((hs) => {
        if (!cancelled) setHelper(hs);
      })
      .catch(() => {
        /* Older backend without the helper command: stay silent. */
      });
    configPaths(ipc)
      .then((p) => {
        if (!cancelled) setPaths(p);
      })
      .catch(() => {
        /* V3-26: fall back to the documented default, never a wrong path. */
      });
    return () => {
      cancelled = true;
    };
  }, [ipc, routingBusy]);

  async function handleCheckRoutingHealth() {
    setCheckingHealth(true);
    setRoutingError(null);
    try {
      setRoutingHealth(await checkRoutingHealth(ipc));
    } catch (err: unknown) {
      setRoutingError(
        isCommandError(err) ? err.message : "The routing targets could not be checked.",
      );
    } finally {
      setCheckingHealth(false);
    }
  }

  async function handleSaveRouting() {
    if (routing === null) return;
    setRoutingBusy(true);
    setRoutingError(null);
    try {
      await saveRouting(routing, ipc);
    } catch (err: unknown) {
      setRoutingError(
        isCommandError(err) ? err.message : "The routing chain could not be saved.",
      );
    } finally {
      setRoutingBusy(false);
    }
  }

  async function handleStartProxy() {
    setRoutingBusy(true);
    setRoutingError(null);
    try {
      await proxyStart(ipc);
      setProxy(await proxyStatus(ipc));
    } catch (err: unknown) {
      setRoutingError(isCommandError(err) ? err.message : "The proxy could not be started.");
    } finally {
      setRoutingBusy(false);
    }
  }

  async function handleStopProxy() {
    setRoutingBusy(true);
    setRoutingError(null);
    try {
      await proxyStop(ipc);
      setProxy(await proxyStatus(ipc));
    } catch (err: unknown) {
      setRoutingError(isCommandError(err) ? err.message : "The proxy could not be stopped.");
    } finally {
      setRoutingBusy(false);
    }
  }

  async function handleRestartProxy() {
    setRoutingBusy(true);
    setRoutingError(null);
    try {
      await proxyRestart(ipc);
      setProxy(await proxyStatus(ipc));
      setHelper(await helperStatus(ipc));
    } catch (err: unknown) {
      setRoutingError(
        isCommandError(err) ? err.message : "The proxy helper could not be restarted.",
      );
    } finally {
      setRoutingBusy(false);
    }
  }

  async function handleToggleLoginStart(enabled: boolean) {
    setRoutingBusy(true);
    setRoutingError(null);
    try {
      setHelper(await helperLoginStart(enabled, ipc));
    } catch (err: unknown) {
      setRoutingError(
        isCommandError(err)
          ? err.message
          : enabled
            ? "The login agent could not be installed."
            : "The login agent could not be removed.",
      );
    } finally {
      setRoutingBusy(false);
    }
  }

  async function handleApplyRouted() {
    setRoutingBusy(true);
    setRoutingError(null);
    try {
      const outcome = await applyRoutedMode(selectedModel, ipc);
      setApplyOutcome(outcome);
      setProxy(await proxyStatus(ipc));
      setStatus(await readStatus(ipc));
    } catch (err: unknown) {
      setRoutingError(
        isCommandError(err) ? err.message : "Claude Code could not be pointed at the proxy.",
      );
    } finally {
      setRoutingBusy(false);
    }
  }

  async function handleFetchModels() {
    if (selected === null || isCustom) return;
    setFetchingModels(true);
    setModelsError(null);
    setModelsFetchedFor(null);
    setSelectedModel(null); // a fresh catalog never inherits a selection
    try {
      // Pass the typed key only if the catalog needs it; otherwise the
      // backend resolves the stored credential itself.
      const entries = await discoverModels(selected.id, apiKey.trim() || undefined, ipc);
      setModels(entries);
      setModelsFetchedFor(selected.id);
    } catch (err: unknown) {
      setModelsError(isCommandError(err) ? err.message : "Model discovery failed.");
    } finally {
      setFetchingModels(false);
    }
  }

  async function handleTestConnection() {
    if (selected === null || isCustom) return;
    setTestingConnection(true);
    setConnectionTest(null);
    setConnectionTestError(null);
    try {
      const result = await testConnection(selected.id, apiKey.trim() || undefined, ipc);
      setConnectionTest(result);
    } catch (err: unknown) {
      setConnectionTestError(
        isCommandError(err) ? err.message : "The connection test could not be completed.",
      );
    } finally {
      setTestingConnection(false);
    }
  }

  useEffect(() => {
    let cancelled = false;
    checkEnvironment(ipc)
      .then((env) => {
        if (!cancelled) setEnvironment(env);
      })
      .catch(() => {
        /* Environment probe is best-effort. */
      });
    // D9: the Maintenance screen lists real backups; refreshed after restores.
    listBackups(ipc)
      .then((b) => {
        if (!cancelled) setBackups(b);
      })
      .catch(() => {
        /* best-effort */
      });
    return () => {
      cancelled = true;
    };
  }, [ipc, restoreOutcome]);

  useEffect(() => {
    let cancelled = false;
    readStatus(ipc)
      .then((s) => {
        if (!cancelled) setStatus(s);
      })
      .catch(() => {
        /* Status is best-effort; a failed read keeps the honest empty state. */
      });
    return () => {
      cancelled = true;
    };
  }, [ipc, applyOutcome]);

  // V2-12a: provider switch asks the backend once whether a key is on disk,
  // so "change model only" never forces a retype and never implies a key
  // exists when it does not.
  useEffect(() => {
    let cancelled = false;
    if (selectedId === "" || selectedId === EMPTY_ID || selectedId === CUSTOM_ID)
      return () => {
        cancelled = true;
      };
    hasCredential(selectedId, ipc)
      .then((found) => {
        if (!cancelled) setCredentialCheck({ providerId: selectedId, has: found });
      })
      .catch(() => {
        if (!cancelled) setCredentialCheck({ providerId: selectedId, has: false });
      });
    return () => {
      cancelled = true;
    };
  }, [selectedId, ipc]);

  // V2-21: reload the masked credential list whenever the provider changes or a
  // mutation succeeded. A failure here must never block the rest of the screen.
  const reloadCredentials = useCallback(async () => {
    if (selectedId === "" || selectedId === EMPTY_ID || selectedId === CUSTOM_ID) return;
    try {
      const list = await listCredentials(selectedId, ipc);
      setCredentials({ providerId: selectedId, list });
      setCredentialsError(null);
    } catch (err: unknown) {
      setCredentials(null);
      setCredentialsError(isCommandError(err) ? err.message : "Could not read stored keys.");
    }
  }, [selectedId, ipc]);

  useEffect(() => {
    if (selectedId === "" || selectedId === EMPTY_ID || selectedId === CUSTOM_ID) return;
    let cancelled = false;
    listCredentials(selectedId, ipc)
      .then((list) => {
        if (cancelled) return;
        setCredentials({ providerId: selectedId, list });
        setCredentialsError(null);
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        setCredentials(null);
        setCredentialsError(isCommandError(err) ? err.message : "Could not read stored keys.");
      });
    return () => {
      cancelled = true;
    };
  }, [selectedId, ipc]);

  async function handleAddCredential(label: string, secret: string) {
    if (selectedId === "" || selectedId === EMPTY_ID || selectedId === CUSTOM_ID) return;
    try {
      await addCredential(selectedId, label, secret, ipc);
      setCredentialsError(null);
      await reloadCredentials();
      setCredentialCheck({ providerId: selectedId, has: true });
    } catch (err: unknown) {
      setCredentialsError(isCommandError(err) ? err.message : "Could not store the key.");
    }
  }

  async function handleRemoveCredential(id: string) {
    try {
      await removeCredential(id, ipc);
      await reloadCredentials();
    } catch (err: unknown) {
      setCredentialsError(isCommandError(err) ? err.message : "Could not delete the key.");
    }
  }

  async function handleToggleCredential(id: string, enabled: boolean) {
    try {
      await setCredentialEnabled(id, enabled, ipc);
      await reloadCredentials();
    } catch (err: unknown) {
      setCredentialsError(isCommandError(err) ? err.message : "Could not update the key.");
    }
  }

  async function handlePromoteCredential(id: string) {
    try {
      await promoteCredential(id, ipc);
      await reloadCredentials();
    } catch (err: unknown) {
      setCredentialsError(isCommandError(err) ? err.message : "Could not reorder the keys.");
    }
  }

  // D2: a persisted provider id must still exist in the catalog; otherwise
  // the UI would show "configured" while no provider is actually selected.
  useEffect(() => {
    if (providers === null) return;
    if (selectedId === EMPTY_ID || selectedId === CUSTOM_ID) return;
    if (!providers.some((p) => p.id === selectedId)) {
      setSelectedId(EMPTY_ID);
      clearSessionString("provider");
    }
  }, [providers, selectedId]);

  useEffect(() => {
    let cancelled = false;
    listProviders(ipc)
      .then((p) => {
        if (!cancelled) {
          setProviders(p);
          setLoadError(null);
        }
      })
      .catch((err: unknown) => {
        if (!cancelled) {
          setLoadError(isCommandError(err) ? err.message : "Provider catalog failed to load.");
        }
      });
    return () => {
      cancelled = true;
    };
  }, [ipc]);

  // A shell export beats everything this app writes; a GUI app cannot see the
  // shell, so the backend asks the login shell for names and lengths only.
  const shellShadowing = (environment?.shellEnv ?? [])
    .filter((v) => v.length > 0)
    .map((v) => `${v.name} (${v.length} chars)`);

  const isCustom = selectedId === CUSTOM_ID;
  const selected = providers?.find((p) => p.id === selectedId) ?? null;

  // T43: one validation result drives both the inline message and the
  // Save & Apply gate, so the UI can never offer an apply we would reject.
  const modelValidation = selectedModel === null ? null : validateModelId(selectedModel);
  const modelInvalid = modelValidation !== null && !modelValidation.ok;
  const roleInvalid = hasInvalidRole(roles);

  // Only a result for the CURRENT provider counts; a stale answer from the
  // previous selection must never enable Save.
  const storedCredential = credentialCheck?.providerId === selectedId && credentialCheck.has;

  // Only a list loaded for THIS provider counts; a stale list must never be
  // shown against a different provider.
  const credentialList = credentials?.providerId === selectedId ? credentials.list : [];

  // V2-21: the first enabled credential is the one the helper will read.
  const activeCredential = credentialList.find((c) => c.enabled) ?? null;

  const contextWindowDigits = contextWindow.trim();
  const contextWindowInvalid =
    contextWindowDigits !== "" &&
    !(Number(contextWindowDigits) >= 1_000 && Number(contextWindowDigits) <= 10_000_000);

  // V3-25: Claude Code is currently pointed at the local proxy. The global
  // button must not silently undo that — routed mode can be working while the
  // draft on screen is a direct-mode provider.
  const routedActive = isLoopback(status?.envBaseUrl);

  const canApply =
    (selected !== null || (isCustom && customEndpoint.trim() !== "")) &&
    (apiKey.trim() !== "" || storedCredential) &&
    !contextWindowInvalid &&
    !modelInvalid &&
    !roleInvalid &&
    !applying;

  // The connection test needs a real provider (a custom endpoint has no
  // documented key-check endpoint) and something to test.
  const testDisabled =
    selected === null ||
    isCustom ||
    (apiKey.trim() === "" && !storedCredential) ||
    testingConnection;

  // D3: the action bar speaks for the screen you are on. Provider-dependent
  // guidance only appears on provider-scoped screens; Maintenance never asks
  // for a provider; Failover's bar is handled by the routed-mode branch.
  const applyHint = (() => {
    if (tab === "maintenance") {
      return "Nothing to apply here — changes happen on the other screens.";
    }
    if (tab === "routing") {
      return "Configure the chain above, then use Route Claude Code.";
    }
    if (canApply) return "Writes ~/.claude/settings.json after a backup.";
    if (selected === null && !isCustom)
      return "Choose a provider to enable Save & Apply.";
    if (apiKey.trim() === "" && !storedCredential)
      return "Add a key (or store one under Credentials) to enable Save & Apply.";
    return "Fix the highlighted field to enable Save & Apply.";
  })();

  async function handleApply() {
    if (!canApply) return;
    setApplying(true);
    setApplyOutcome(null);
    setApplyError(null);
    try {
      const baseUrl = isCustom ? customEndpoint.trim() : (selected?.directBaseUrl ?? undefined);
      const outcome = await saveAndApply(
        {
          providerId: isCustom ? "custom" : (selected?.id ?? ""),
          baseUrl,
          // Only a validated (trimmed) id is ever sent; blank means "no model".
          model: modelValidation?.ok ? modelValidation.value : undefined,
          // T45: only filled-in roles are sent; the rest stay untouched.
          roleModels: rolesToSend(roles),
          // The helper must read the selected credential, not a provider default.
          credentialId: activeCredential?.id,
          // Blank means "leave the managed key as it is"; the reset flag is
          // only sent when an effective value exists and the field was cleared.
          maxContextTokens: contextWindowDigits === "" ? undefined : Number(contextWindowDigits),
          resetContextWindow:
            contextWindowDigits === "" && (status?.envMaxContextTokens ?? null) !== null,
          // Blank keeps the stored credential untouched (backend contract).
          apiKey: apiKey.trim(),
        },
        ipc,
      );
      setApplyOutcome(outcome);
    } catch (err: unknown) {
      setApplyError(isCommandError(err) ? err.message : "Apply failed.");
    } finally {
      setApplying(false);
    }
  }

  async function handleRestore() {
    setRestoring(true);
    setRestoreOutcome(null);
    setRestoreError(null);
    try {
      const outcome = await restoreLastBackup(ipc);
      setRestoreOutcome(outcome);
    } catch (err: unknown) {
      setRestoreError(isCommandError(err) ? err.message : "Restore failed.");
    } finally {
      setRestoring(false);
    }
  }

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-app text-ink">
      <AppHeader
        status={status}
        connection={{ proxyRunning: helper?.running ?? proxy !== null, lastTest: connectionTest }}
      />
      <div className="flex min-h-0 flex-1">
        {/* macOS settings shape: a fixed, quiet sidebar and one scrolling panel.
            The accent appears only on the current item's glyph. */}
        <aside className="flex w-[11.75rem] shrink-0 flex-col border-r border-line bg-panel/80 pb-3 backdrop-blur">
          <AppSidebar
            active={tab}
            onSelect={navigate}
            badges={{
              credentials: credentialList.length > 0 ? String(credentialList.length) : undefined,
              routing: routing?.enabled === true ? "on" : undefined,
            }}
          />
          <ThemeSegmented />
        </aside>

        <main className="flex min-h-0 min-w-0 flex-1 flex-col break-words">
          <div className="flex-1 overflow-y-auto">
            <div className="mx-auto flex w-full max-w-[52rem] flex-col gap-6 px-6 pt-5 pb-6">
              <header>
                <h2 className="text-[19px] leading-[1.25] font-semibold tracking-[-0.01em]">
                  {TAB_META[tab].title}
                </h2>
                <p className="mt-1.5 text-[12.5px] leading-[1.5] text-ink-muted">
                  {TAB_META[tab].purpose}
                </p>
              </header>

              {loadError !== null && (
                <StatusBanner
                  severity="error"
                  message="Could not load the provider catalog."
                  detail={loadError}
                />
              )}

              {environment !== null && shellShadowing.length > 0 && !shellWarningDismissed && (
                <StatusBanner
                  severity="warning"
                  message="Your shell exports ANTHROPIC settings that override this app."
                  detail={`${shellShadowing.join(", ")} — Claude Code ranks these above apiKeyHelper and settings.json. Unset them in ~/.zshrc (or your shell profile) and open a new terminal.`}
                  onDismiss={() => setShellWarningDismissed(true)}
                />
              )}

              {environment !== null &&
                (environment.settingsState === "malformed_json" ||
                  environment.settingsState === "symlink") && (
                  <StatusBanner
                    severity="error"
                    message="The settings file has a problem this app will not overwrite."
                    detail={environment.settingsDetail ?? undefined}
                  />
                )}

              {tab === "dashboard" && (
                <DashboardTab
                  status={status}
                  providers={providers}
                  selected={selected}
                  credentials={credentialList}
                  routing={routing}
                  proxy={proxy}
                  environment={environment}
                  applyOutcome={applyOutcome}
                  applyError={applyError}
                  onDismissApply={() => setApplyOutcome(null)}
                  onDismissApplyError={() => setApplyError(null)}
                  onNavigate={navigate}
                />
              )}

              {tab === "provider" && (
                <ProviderTab
                  providers={providers}
                  selectedId={selectedId}
                  onSelectProvider={(id) => {
                    setSelectedId(id);
                    if (id === EMPTY_ID || id === CUSTOM_ID) clearSessionString("provider");
                    else saveSessionString("provider", id);
                    // A model id from another provider must never carry over.
                    setSelectedModel(null);
                    setModels(null);
                    setModelsFetchedFor(null);
                    setModelsError(null);
                  }}
                  selected={selected}
                  isCustom={isCustom}
                  customName={customName}
                  onCustomName={setCustomName}
                  customEndpoint={customEndpoint}
                  onCustomEndpoint={setCustomEndpoint}
                  apiKey={apiKey}
                  onApiKey={setApiKey}
                  storedCredential={storedCredential}
                  emptyId={EMPTY_ID}
                  customId={CUSTOM_ID}
                  testing={testingConnection}
                  testDisabled={testDisabled}
                  onTestConnection={() => {
                    void handleTestConnection();
                  }}
                  testResult={connectionTest}
                  testError={connectionTestError}
                  onDismissTest={() => setConnectionTest(null)}
                  onDismissTestError={() => setConnectionTestError(null)}
                />
              )}

              {tab === "credentials" && (
                <CredentialsTab
                  onNavigateToProvider={() => navigate("provider")}
                  provider={selected}
                  isCustom={isCustom}
                  credentials={credentialList}
                  error={credentialsError}
                  addDisabled={selected === null && !isCustom}
                  onAdd={(label, secret) => {
                    void handleAddCredential(label, secret);
                  }}
                  onRemove={(id) => {
                    void handleRemoveCredential(id);
                  }}
                  onToggle={(id, enabled) => {
                    void handleToggleCredential(id, enabled);
                  }}
                  onPromote={(id) => {
                    void handlePromoteCredential(id);
                  }}
                  secretsDir={paths?.secretsDir ?? null}
                />
              )}

              {tab === "model" && (
                <ModelTab
                  provider={selected}
                  isCustom={isCustom}
                  models={models}
                  fetching={fetchingModels}
                  fetchedFor={modelsFetchedFor}
                  onFetch={() => {
                    void handleFetchModels();
                  }}
                  modelsError={modelsError}
                  onDismissModelsError={() => setModelsError(null)}
                  selectedModel={selectedModel}
                  onSelectModel={setSelectedModel}
                  modelInvalid={modelInvalid}
                  modelMessage={
                    modelValidation !== null && !modelValidation.ok ? modelValidation.message : null
                  }
                  status={status}
                  roles={roles}
                  onRole={(key, value) => setRoles((r) => ({ ...r, [key]: value }))}
                  roleInvalid={roleInvalid}
                  advancedOpen={advancedOpen}
                  onToggleAdvanced={() => setAdvancedOpen((v) => !v)}
                  contextWindow={contextWindow}
                  onContextWindow={setContextWindow}
                />
              )}

              {tab === "routing" && (
                <RoutingTab
                  config={routing}
                  error={routingError}
                  providers={providers ?? []}
                  credentials={
                    routingCredentials !== null &&
                    (routingCredentials.providerId === routingDraftProviderId ||
                      routingDraftProviderId === null)
                      ? routingCredentials.list
                      : []
                  }
                  onNeedCredentials={(providerId) => {
                    setRoutingDraftProviderId(providerId);
                    void listCredentials(providerId, ipc)
                      .then((list) => setRoutingCredentials({ providerId, list }))
                      .catch(() => setRoutingCredentials(null));
                  }}
                  status={proxy}
                  health={routingHealth}
                  checking={checkingHealth}
                  busy={routingBusy}
                  onChange={setRouting}
                  onSave={() => {
                    void handleSaveRouting();
                  }}
                  onStartProxy={() => {
                    void handleStartProxy();
                  }}
                  onStopProxy={() => {
                    void handleStopProxy();
                  }}
                  helper={helper}
                  onRestartProxy={() => {
                    void handleRestartProxy();
                  }}
                  onToggleLoginStart={(enabled: boolean) => {
                    void handleToggleLoginStart(enabled);
                  }}
                  onApplyRouted={() => {
                    void handleApplyRouted();
                  }}
                  onCheckHealth={() => {
                    void handleCheckRoutingHealth();
                  }}
                />
              )}

              {tab === "maintenance" && (
                <MaintenanceTab
                  backups={backups}
                  environment={environment}
                  restoring={restoring}
                  onRestore={() => {
                    void handleRestore();
                  }}
                  restoreOutcome={restoreOutcome}
                  restoreError={restoreError}
                  onDismissRestore={() => setRestoreOutcome(null)}
                  onDismissRestoreError={() => setRestoreError(null)}
                  shellWarningDismissed={shellWarningDismissed}
                  onDismissShellWarning={() => setShellWarningDismissed(true)}
                />
              )}
            </div>
          </div>

          {/* One primary action, always reachable, with the reason when it cannot run. */}
          <div className="border-t border-line bg-app px-6 py-3">
            <div className="mx-auto flex max-w-[52rem] items-center gap-3">
              {routedActive ? (
                confirmingDirect ? (
                  <>
                    <p className="mr-auto text-[12px] text-ink-muted">
                      Switch Claude Code back to a direct provider? The proxy keeps running, so
                      you can switch back at any time.
                    </p>
                    <button
                      type="button"
                      onClick={() => setConfirmingDirect(false)}
                      className="h-8 rounded-[6px] border border-control bg-surface px-3 text-[12.5px] text-ink hover:bg-surface-hover"
                    >
                      Cancel
                    </button>
                    <LoadingButton
                      label="Switch to direct"
                      pendingLabel="Applying…"
                      pending={applying}
                      disabled={!canApply}
                      variant="primary"
                      onClick={() => {
                        void handleApply().then(() => setConfirmingDirect(false));
                      }}
                    />
                  </>
                ) : (
                  <>
                    <p className="mr-auto text-[12px] text-ink-muted">
                      Claude Code is routed to the local proxy.
                    </p>
                    <button
                      type="button"
                      onClick={() => setConfirmingDirect(true)}
                      className="h-8 rounded-[6px] border border-control bg-surface px-3 text-[12.5px] text-ink hover:bg-surface-hover"
                    >
                      Switch to direct…
                    </button>
                    <LoadingButton
                      label="Apply routed mode"
                      pendingLabel="Applying…"
                      pending={applying}
                      disabled={routingBusy || routing === null || !routing.enabled}
                      variant="primary"
                      onClick={() => {
                        void handleApplyRouted();
                      }}
                    />
                  </>
                )
              ) : (
                <>
                  <p className="mr-auto text-[12px] text-ink-muted">{applyHint}</p>
                  <LoadingButton
                    label="Save & Apply"
                    pendingLabel="Applying…"
                    pending={applying}
                    disabled={!canApply}
                    variant="primary"
                    onClick={() => {
                      void handleApply();
                    }}
                  />
                </>
              )}
            </div>
          </div>
        </main>
      </div>
    </div>
  );
}
