import { useCallback, useEffect, useRef, useState } from "react";
import { loadProviderDraft, saveProviderDraft, customProviderDrafts } from "./lib/providerDraft";
import { helperCommandPath } from "./lib/helperCommand";
import { FieldRow } from "./components/FieldRow";
import { LoadingButton } from "./components/LoadingButton";
import { EMPTY_ROLES, hasInvalidRole, rolesToSend, type RoleState } from "./lib/roles";
import { AppHeader } from "./components/AppHeader";
import { AppSidebar } from "./components/AppSidebar";
import { ThemeSegmented } from "./components/ThemeSegmented";
import { DashboardTab } from "./tabs/DashboardTab";
import { ProviderTab } from "./tabs/ProviderTab";
import { CredentialsTab } from "./tabs/CredentialsTab";
import { ModelTab } from "./tabs/ModelTab";
import { RoutingTab } from "./tabs/RoutingTab";
import { BridgeCard } from "./components/BridgeCard";
import { RecoverySelector } from "./components/RecoverySelector";
import { MaintenanceTab } from "./tabs/MaintenanceTab";
import { TAB_META, isTabId, type TabId } from "./lib/tab";
import { loadSessionString, saveSessionString, clearSessionString } from "./lib/session";
import { StatusBanner } from "./components/StatusBanner";
import {
  checkEnvironment,
  defaultIpc,
  listBackups,
  backupPreviews,
  pendingTransaction,
  recoverTransaction,
  type PendingTransaction,
  type BackupPreview,
  discoverModels,
  testConnection,
  addCredential,
  listCredentials,
  promoteCredential,
  removeCredential,
  setCredentialEnabled,
  listProviders,
  readStatus,
  restoreLastBackup,
  recoverSettings,
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
import { classifyEndpoint, customProviderId } from "./lib/endpoint";
import { HelpPopover } from "./components/HelpPopover";

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
  const selectedIdRef = useRef(selectedId);
  useEffect(() => {
    selectedIdRef.current = selectedId;
  }, [selectedId]);
  const [customName, setCustomName] = useState(() => loadProviderDraft(selectedId).customName);
  const [customEndpoint, setCustomEndpoint] = useState(
    () => loadProviderDraft(selectedId).customEndpoint,
  );
  const [apiKey, setApiKey] = useState("");
  const [applying, setApplying] = useState(false);
  const [applyOutcome, setApplyOutcome] = useState<ApplyOutcome | null>(null);
  const [applyError, setApplyError] = useState<string | null>(null);
  const [status, setStatus] = useState<EffectiveStatus | null>(null);
  const [savedCredentials, setSavedCredentials] = useState<CredentialView[]>([]);
  const [restoring, setRestoring] = useState(false);
  const [restoreOutcome, setRestoreOutcome] = useState<RestoreOutcome | null>(null);
  const [restoreError, setRestoreError] = useState<string | null>(null);
  const [environment, setEnvironment] = useState<EnvironmentReport | null>(null);
  const [environmentError, setEnvironmentError] = useState<string | null>(null);
  const [interrupted, setInterrupted] = useState<PendingTransaction | null>(null);
  const [interruptedError, setInterruptedError] = useState<string | null>(null);
  const [recoveringOperation, setRecoveringOperation] = useState(false);
  const [operationChoice, setOperationChoice] = useState<"restore" | "keep" | null>(null);
  const [backupChoices, setBackupChoices] = useState<BackupPreview[] | null>(null);
  const [backupChoiceError, setBackupChoiceError] = useState<string | null>(null);
  const [selectedBackup, setSelectedBackup] = useState<string | null>(null);
  const [backups, setBackups] = useState<BackupInfo[] | null>(null);
  const [shellWarningDismissed, setShellWarningDismissed] = useState(false);
  const [models, setModels] = useState<ModelEntry[] | null>(null);
  const [fetchingModels, setFetchingModels] = useState(false);
  const [modelsError, setModelsError] = useState<string | null>(null);
  const [modelsFetchedFor, setModelsFetchedFor] = useState<string | null>(null);
  const [testingConnection, setTestingConnection] = useState(false);
  const [connectionTest, setConnectionTest] = useState<ConnectionTestResult | null>(null);
  const [connectionTestError, setConnectionTestError] = useState<string | null>(null);
  const [selectedModel, setSelectedModel] = useState<string | null>(
    () => loadProviderDraft(selectedId).model,
  );
  // T45: role overrides start empty = "no override" and stay collapsed.
  const [roles, setRoles] = useState<RoleState>(() => loadProviderDraft(selectedId).roles);
  const [resetRoles, setResetRoles] = useState<string[]>(
    () => loadProviderDraft(selectedId).resetRoles,
  );
  const [advancedOpen, setAdvancedOpen] = useState(false);
  // One screen at a time: the sidebar swaps the panel, never adds to it.
  // D1: the last screen is restored on relaunch.
  const [tab, setTab] = useState<TabId>(() => {
    const saved = loadSessionString("tab");
    return saved === "credentials" || saved === "model"
      ? "provider"
      : isTabId(saved ?? "")
        ? (saved as TabId)
        : "dashboard";
  });
  // D1: every navigation is remembered for the next launch.
  const navigate = useCallback((next: TabId) => {
    const destination = next === "credentials" || next === "model" ? "provider" : next;
    setTab(destination);
    saveSessionString("tab", destination);
  }, []);
  // V2-12b: declared window for models Claude Code can't recognize.
  const [contextWindow, setContextWindow] = useState(() => loadProviderDraft(selectedId).context);
  const [resetContext, setResetContext] = useState(
    () => loadProviderDraft(selectedId).resetContext,
  );
  const [refresh, setRefresh] = useState(0);
  const [statusError, setStatusError] = useState<string | null>(null);
  const [recovering, setRecovering] = useState(false);
  const [confirmRecovery, setConfirmRecovery] = useState<"backup" | "clean" | null>(null);

  // V2-12a: a stored credential means the key field can stay blank.
  // V2-21: masked credential list for the selected provider.
  const [credentials, setCredentials] = useState<{
    providerId: string;
    list: CredentialView[];
  } | null>(null);
  const [credentialsError, setCredentialsError] = useState<string | null>(null);
  // V2-23/30: failover chain and proxy state, independent of the provider pick.
  const [routing, setRouting] = useState<RoutingConfig | null>(null);
  const [savedRouting, setSavedRouting] = useState<string | null>(null);
  const [routingReceipt, setRoutingReceipt] = useState<string | null>(null);
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

  const isCustom = selectedId === CUSTOM_ID;
  const selected = providers?.find((p) => p.id === selectedId) ?? null;
  const customCheck = classifyEndpoint(customEndpoint);
  const customValid = isCustom && ["remote_ok", "local"].includes(customCheck.check);
  const credentialProviderId = isCustom
    ? customValid
      ? customProviderId(customEndpoint)
      : null
    : (selected?.id ?? null);
  useEffect(() => {
    saveProviderDraft(
      selectedId,
      {
        model: selectedModel,
        roles,
        context: contextWindow,
        resetContext,
        resetRoles,
        customName,
        customEndpoint,
      },
      isCustom ? (credentialProviderId ?? undefined) : undefined,
    );
  }, [
    selectedId,
    selectedModel,
    roles,
    contextWindow,
    resetContext,
    resetRoles,
    customName,
    customEndpoint,
    isCustom,
    credentialProviderId,
  ]);
  const scopeRef = useRef(credentialProviderId);
  useEffect(() => {
    scopeRef.current = credentialProviderId;
  }, [credentialProviderId]);
  useEffect(() => {
    const update = () => setRefresh((n) => n + 1);
    window.addEventListener("focus", update);
    return () => window.removeEventListener("focus", update);
  }, []);

  useEffect(() => {
    let cancelled = false;
    getRouting(ipc)
      .then((cfg) => {
        if (!cancelled) {
          setRouting(cfg);
          setSavedRouting(JSON.stringify(cfg));
        }
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

  useEffect(() => {
    if (tab !== "routing" && tab !== "dashboard") return;
    let cancelled = false;
    const timer = window.setInterval(() => {
      void proxyStatus(ipc)
        .then((value) => {
          if (!cancelled) setProxy(value);
        })
        .catch(() => {});
      void helperStatus(ipc)
        .then((value) => {
          if (!cancelled) setHelper(value);
        })
        .catch(() => {});
    }, 5000);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [tab, ipc]);

  async function handleCheckRoutingHealth() {
    setCheckingHealth(true);
    setRoutingError(null);
    try {
      if (routing) {
        await saveRouting(routing, ipc);
        setSavedRouting(JSON.stringify(routing));
        setRoutingReceipt("Visible policy saved before checking keys.");
      }
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
      setSavedRouting(JSON.stringify(routing));
      setRoutingReceipt(
        "Switching order saved. Activation is a separate action when using a direct connection.",
      );
    } catch (err: unknown) {
      setRoutingError(isCommandError(err) ? err.message : "The routing chain could not be saved.");
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
      if (!routing) return;
      const next = { ...routing, enabled: true };
      const outcome = await applyRoutedMode(null, ipc, next);
      setRouting(next);
      setSavedRouting(JSON.stringify(next));
      setRoutingReceipt("Switching policy saved and activated.");
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

  // Supersede in-flight catalog/connection requests when the provider
  // changes — a resolved response for provider A must never render as
  // provider B's result.
  const fetchModelsSeq = useRef(0);
  const testConnSeq = useRef(0);

  async function handleFetchModels() {
    if (selected === null || isCustom) return;
    const seq = ++fetchModelsSeq.current;
    const forProvider = selected.id;
    setFetchingModels(true);
    setModelsError(null);
    setModelsFetchedFor(null);
    try {
      // Pass the typed key only if the catalog needs it; otherwise the
      // backend resolves the stored credential itself.
      const entries = await discoverModels(forProvider, apiKey.trim() || undefined, ipc);
      if (seq !== fetchModelsSeq.current) return; // provider switched mid-flight
      setModels(entries);
      setModelsFetchedFor(forProvider);
    } catch (err: unknown) {
      if (seq !== fetchModelsSeq.current) return;
      setModelsError(isCommandError(err) ? err.message : "Model discovery failed.");
    } finally {
      if (seq === fetchModelsSeq.current) setFetchingModels(false);
    }
  }

  async function handleTestConnection() {
    if (selected === null || isCustom) return;
    const seq = ++testConnSeq.current;
    setTestingConnection(true);
    setConnectionTest(null);
    setConnectionTestError(null);
    try {
      const result = await testConnection(selected.id, apiKey.trim() || undefined, ipc);
      if (seq !== testConnSeq.current) return; // provider switched mid-flight
      setConnectionTest(result);
    } catch (err: unknown) {
      if (seq !== testConnSeq.current) return;
      setConnectionTestError(
        isCommandError(err) ? err.message : "The connection test could not be completed.",
      );
    } finally {
      if (seq === testConnSeq.current) setTestingConnection(false);
    }
  }

  useEffect(() => {
    let cancelled = false;
    checkEnvironment(ipc)
      .then((env) => {
        if (!cancelled) {
          setEnvironment(env);
          setEnvironmentError(null);
        }
      })
      .catch((e: unknown) => {
        if (!cancelled) {
          setEnvironmentError(isCommandError(e) ? e.message : "Environment check failed.");
        }
      });
    backupPreviews(ipc)
      .then((choices) => {
        if (cancelled) return;
        setBackupChoices(choices);
        setBackupChoiceError(null);
        setSelectedBackup((selected) =>
          choices.some((b) => b.valid && b.fileName === selected)
            ? selected
            : (choices.find((b) => b.valid)?.fileName ?? null),
        );
      })
      .catch((err: unknown) => {
        if (!cancelled) {
          setBackupChoiceError(
            isCommandError(err) ? err.message : "Backup previews could not be read.",
          );
          setBackupChoices([]);
        }
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
  }, [ipc, restoreOutcome, applyOutcome, refresh]);

  useEffect(() => {
    let cancelled = false;
    readStatus(ipc)
      .then((s) => {
        if (!cancelled) {
          setStatus(s);
          setStatusError(null);
        }
      })
      .catch((err: unknown) => {
        if (!cancelled) {
          setStatusError(
            isCommandError(err) ? err.message : "The saved configuration could not be read.",
          );
          setStatus(null);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [ipc, applyOutcome, restoreOutcome, refresh]);

  // V2-21: reload the masked credential list whenever the provider changes or a
  // mutation succeeded. A failure here must never block the rest of the screen.
  const reloadCredentials = useCallback(async () => {
    if (!credentialProviderId) return;
    try {
      const list = await listCredentials(credentialProviderId, ipc);
      if (scopeRef.current !== credentialProviderId) return;
      setCredentials({ providerId: credentialProviderId, list });
      setCredentialsError(null);
    } catch (err: unknown) {
      if (scopeRef.current !== credentialProviderId) return;
      setCredentials(null);
      setCredentialsError(isCommandError(err) ? err.message : "Could not read stored keys.");
    }
  }, [credentialProviderId, ipc]);
  useEffect(() => {
    if (!credentialProviderId) return;
    let cancelled = false;
    listCredentials(credentialProviderId, ipc)
      .then((list) => {
        if (!cancelled) {
          setCredentials({ providerId: credentialProviderId, list });
          setCredentialsError(null);
        }
      })
      .catch((err: unknown) => {
        if (!cancelled) {
          setCredentials(null);
          setCredentialsError(isCommandError(err) ? err.message : "Could not read stored keys.");
        }
      });
    return () => {
      cancelled = true;
    };
  }, [credentialProviderId, ipc, refresh]);

  async function handleAddCredential(label: string, secret: string): Promise<boolean> {
    if (!credentialProviderId) return false;
    try {
      await addCredential(credentialProviderId, label, secret, ipc);
      if (scopeRef.current === credentialProviderId) {
        await reloadCredentials();
        setCredentialsError(null);
      }
      return true;
    } catch (err: unknown) {
      if (scopeRef.current === credentialProviderId)
        setCredentialsError(isCommandError(err) ? err.message : "Could not store the key.");
      return false;
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

  useEffect(() => {
    let cancelled = false;
    listProviders(ipc)
      .then((p) => {
        if (!cancelled) {
          setProviders(p);
          setLoadError(null);
          // D2: a persisted provider id must still exist in the catalog;
          // otherwise the UI would show "configured" while nothing is
          // actually selected. (Async continuation — not sync-in-effect.)
          if (
            selectedIdRef.current !== EMPTY_ID &&
            selectedIdRef.current !== CUSTOM_ID &&
            !p.some((x) => x.id === selectedIdRef.current)
          ) {
            setSelectedId(EMPTY_ID);
            clearSessionString("provider");
          }
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

  // T43: one validation result drives both the inline message and the
  // Save & Apply gate, so the UI can never offer an apply we would reject.
  const modelValidation = selectedModel === null ? null : validateModelId(selectedModel);
  const modelInvalid = modelValidation !== null && !modelValidation.ok;
  const roleInvalid = hasInvalidRole(roles);

  // Only a result for the CURRENT provider counts; a stale answer from the
  // previous selection must never enable Save.
  const storedCredential =
    credentials?.providerId === credentialProviderId && credentials.list.some((key) => key.enabled);

  // Only a list loaded for THIS provider counts; a stale list must never be
  // shown against a different provider.
  const credentialList = credentials?.providerId === credentialProviderId ? credentials.list : [];

  // V2-21: the first enabled credential is the one the helper will read.
  const activeCredential = credentialList.find((c) => c.enabled) ?? null;

  const contextWindowDigits = contextWindow.trim();
  const contextWindowInvalid =
    contextWindowDigits !== "" &&
    !(Number(contextWindowDigits) >= 1_000 && Number(contextWindowDigits) <= 10_000_000);

  // D2 (extended): when the session has no explicit selection but the live
  // settings point at one of our helpers, the provider is knowable from the
  // helper file name — the backend's own record. Shared by every card so the
  // Dashboard can never contradict itself.
  const configuredProvider =
    status?.apiKeyHelper != null && providers !== null
      ? (providers.find(
          (p) =>
            p.id ===
            (helperCommandPath(status.apiKeyHelper!) ?? "").split("/").pop()?.replace(/\.sh$/, ""),
        ) ?? null)
      : null;

  const savedProviderId = configuredProvider?.id;
  useEffect(() => {
    if (!savedProviderId) return;
    let cancelled = false;
    listCredentials(savedProviderId, ipc)
      .then((list) => {
        if (!cancelled) setSavedCredentials(list);
      })
      .catch(() => {
        if (!cancelled) setSavedCredentials([]);
      });
    return () => {
      cancelled = true;
    };
  }, [savedProviderId, ipc, refresh, applyOutcome]);

  // V3-25: Claude Code is currently pointed at the local proxy. The global
  // button must not silently undo that — routed mode can be working while the
  // draft on screen is a direct-mode provider.
  const routedActive = isLoopback(status?.envBaseUrl);

  const canApply =
    ((selected !== null &&
      (selected.protocol !== "open_ai_only" || selected.id === "nvidia-nim")) ||
      customValid) &&
    (apiKey.trim() !== "" || storedCredential) &&
    (selected?.id !== "nvidia-nim" || Boolean(selectedModel?.trim())) &&
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
    testingConnection ||
    selected?.protocol === "open_ai_only" ||
    selected?.id === "zai";

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
    if (selected?.protocol === "open_ai_only" && selected.id !== "nvidia-nim")
      return `Save & Apply is unavailable for ${selected.name} in this version. Choose a supported provider.`;
    if (selected?.id === "nvidia-nim" && apiKey.trim() === "" && !storedCredential)
      return "Add a NVIDIA key to enable Save & Apply.";
    if (selected?.id === "nvidia-nim")
      return selectedModel?.trim()
        ? "Save activates NVIDIA through the local helper and puts its keys first in switching order."
        : "Choose a NVIDIA model to enable Save & Apply.";
    if (canApply) return "Writes ~/.claude/settings.json after a backup.";
    if (selected === null && !isCustom) return "Choose a provider to enable Save & Apply.";
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
          providerId: credentialProviderId ?? "",
          baseUrl,
          // Only a validated (trimmed) id is ever sent; blank means "no model".
          model: modelValidation?.ok ? modelValidation.value : undefined,
          // T45: only filled-in roles are sent; the rest stay untouched.
          roleModels: rolesToSend(roles),
          resetRoles,
          // The helper must read the selected credential, not a provider default.
          credentialId: activeCredential?.id,
          // Blank means "leave the managed key as it is"; the reset flag is
          // only sent when an effective value exists and the field was cleared.
          maxContextTokens: contextWindowDigits === "" ? undefined : Number(contextWindowDigits),
          resetContextWindow: resetContext,
          // Blank keeps the stored credential untouched (backend contract).
          apiKey: apiKey.trim(),
        },
        ipc,
      );
      setApplyOutcome(outcome);
      if (selected?.id === "nvidia-nim") {
        void Promise.all([getRouting(ipc), proxyStatus(ipc), helperStatus(ipc)])
          .then(([cfg, state, runtime]) => {
            setRouting(cfg);
            setSavedRouting(JSON.stringify(cfg));
            setProxy(state);
            setHelper(runtime);
          })
          .catch(() =>
            setRoutingError(
              "Settings saved, but helper status could not be refreshed. Use Refresh.",
            ),
          );
      }
      setApiKey("");
      setResetContext(false);
      setResetRoles([]);
      await reloadCredentials();
    } catch (err: unknown) {
      setApplyError(isCommandError(err) ? err.message : "Apply failed.");
    } finally {
      setApplying(false);
    }
  }

  useEffect(() => {
    let cancelled = false;
    pendingTransaction(ipc)
      .then((value) => {
        if (!cancelled) {
          setInterrupted(value);
          setInterruptedError(null);
        }
      })
      .catch((err: unknown) => {
        if (!cancelled)
          setInterruptedError(
            isCommandError(err) ? err.message : "Interrupted-save state could not be read.",
          );
      });
    return () => {
      cancelled = true;
    };
  }, [ipc, refresh, applyError, credentialsError, routingError, applyOutcome]);
  async function handleOperationRecovery(restorePrevious: boolean) {
    setRecoveringOperation(true);
    setInterruptedError(null);
    try {
      await recoverTransaction(restorePrevious, ipc);
      setOperationChoice(null);
      setInterrupted(null);
      setRefresh((n) => n + 1);
      setApplyError(null);
      setCredentialsError(null);
      setRoutingError(null);
    } catch (err: unknown) {
      setInterruptedError(
        isCommandError(err)
          ? err.message
          : "Managed-state recovery failed; its journal was retained.",
      );
    } finally {
      setRecoveringOperation(false);
    }
  }

  async function handleRestore() {
    setRestoring(true);
    setRestoreOutcome(null);
    setRestoreError(null);
    try {
      const outcome = await restoreLastBackup(ipc, selectedBackup ?? undefined);
      setRestoreOutcome(outcome);
    } catch (err: unknown) {
      setRestoreError(isCommandError(err) ? err.message : "Restore failed.");
    } finally {
      setRestoring(false);
    }
  }

  async function handleRecovery(useBackup: boolean) {
    setRecovering(true);
    setRestoreError(null);
    try {
      setRestoreOutcome(
        await recoverSettings(
          useBackup,
          ipc,
          useBackup ? (selectedBackup ?? undefined) : undefined,
        ),
      );
      setConfirmRecovery(null);
      setRefresh((n) => n + 1);
    } catch (err: unknown) {
      setRestoreError(
        isCommandError(err) ? err.message : "Recovery failed. Your original file was preserved.",
      );
    } finally {
      setRecovering(false);
    }
  }
  function addProviderGroup() {
    if (!routing || !credentialProviderId || !selectedModel || modelInvalid) return;
    const existing = routing.targets.find(
      (t) => t.providerId === credentialProviderId && t.credentialId === null,
    );
    const target = {
      id:
        existing?.id ??
        (isCustom
          ? `${(customName || "gateway")
              .toLowerCase()
              .replace(/[^a-z0-9]+/g, "-")
              .slice(0, 25)}-${routing.targets.length + 1}`
          : `${credentialProviderId}-group`),
      providerId: credentialProviderId,
      credentialId: null,
      model: selectedModel,
      baseUrl: isCustom
        ? customEndpoint.trim()
        : selected?.id === "nvidia-nim"
          ? "https://integrate.api.nvidia.com"
          : (selected?.directBaseUrl ?? "https://api.anthropic.com"),
      enabled: true,
    };
    setRouting({
      ...routing,
      targets: existing
        ? routing.targets.map((t) => (t.id === existing.id ? target : t))
        : [...routing.targets, target],
    });
    setRoutingHealth(null);
    navigate("routing");
  }

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-app text-ink">
      <AppHeader
        status={status}
        connection={{ proxyRunning: helper?.running ?? proxy !== null, lastTest: null }}
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
                  {TAB_META[tab].title}{" "}
                  <HelpPopover title={TAB_META[tab].title}>
                    {TAB_META[tab].purpose} Saved settings and pending edits are shown separately.
                    Use the activation action to write changes.
                  </HelpPopover>
                </h2>
                <p className="mt-1.5 text-[12.5px] leading-[1.5] text-ink-muted">
                  {TAB_META[tab].purpose}
                </p>
              </header>

              <div className="flex items-center justify-between gap-2 text-[12px] text-ink-muted">
                <span>Global settings · {paths?.settingsFile ?? "~/.claude/settings.json"}</span>
                <button
                  type="button"
                  className="btn btn-secondary !h-7 !px-2"
                  onClick={() => setRefresh((n) => n + 1)}
                >
                  Refresh
                </button>
              </div>
              {statusError && (
                <StatusBanner
                  severity="error"
                  message="Saved settings could not be read."
                  detail={statusError}
                />
              )}
              {configuredProvider?.id === "nvidia-nim" && !routedActive && (
                <StatusBanner
                  severity="warning"
                  message="Saved NVIDIA setup needs activation."
                  detail="The NVIDIA credential helper is saved without the local translation endpoint. Open Providers & keys, choose NVIDIA and a chat model, then Save & Apply to activate its built-in connection."
                />
              )}
              {interruptedError && (
                <StatusBanner
                  severity="error"
                  message="Recovery state could not be read."
                  detail={interruptedError}
                />
              )}
              {interrupted && (
                <section
                  className="rounded-lg border border-warn/30 bg-warn-soft p-4"
                  aria-label="Interrupted save recovery"
                >
                  <h3 className="text-[14px] font-semibold">
                    An interrupted save needs review{" "}
                    <HelpPopover title="Interrupted save recovery">
                      A private before-image journal was written before managed changes. New writes
                      are blocked until you choose whether to restore those files or keep the
                      current state. Recovery never runs silently over later edits.
                    </HelpPopover>
                  </h3>
                  <p className="mt-2 text-[12px]">
                    {interrupted.fileCount} managed files are covered. Restore replaces settings,
                    helper scripts, credential metadata and key files from before the interrupted
                    operation, preserving current bytes in a private recovery snapshot first. Keep
                    current only after reviewing the files; it clears the journal so you can apply a
                    complete configuration again.
                  </p>
                  <div className="mt-3 flex flex-wrap gap-2">
                    <button
                      type="button"
                      className="btn btn-secondary"
                      disabled={recoveringOperation}
                      onClick={() => setOperationChoice("restore")}
                    >
                      Restore previous managed state…
                    </button>
                    <button
                      type="button"
                      className="btn btn-secondary"
                      disabled={recoveringOperation}
                      onClick={() => setOperationChoice("keep")}
                    >
                      Keep current files…
                    </button>
                  </div>
                  {operationChoice && (
                    <div className="mt-3">
                      <p className="text-[12px]">
                        {operationChoice === "restore"
                          ? "Restore the pre-save managed files? This can replace newer manual changes; a private snapshot of current files is preserved first."
                          : "Keep current files and clear the pending journal? This does not verify provider connectivity or guarantee that the interrupted operation completed."}
                      </p>
                      <div className="mt-2 flex gap-2">
                        <button
                          type="button"
                          className="btn btn-primary"
                          disabled={recoveringOperation}
                          onClick={() =>
                            void handleOperationRecovery(operationChoice === "restore")
                          }
                        >
                          {recoveringOperation ? "Recovering…" : "Confirm recovery choice"}
                        </button>
                        <button
                          type="button"
                          className="btn btn-secondary"
                          onClick={() => setOperationChoice(null)}
                        >
                          Cancel
                        </button>
                      </div>
                    </div>
                  )}
                </section>
              )}
              {tab !== "dashboard" && applyOutcome && (
                <StatusBanner
                  severity="success"
                  message="Configuration saved and verified."
                  detail="File verification is complete. Provider connectivity is a separate check."
                  onDismiss={() => setApplyOutcome(null)}
                />
              )}
              {tab !== "dashboard" && applyError && (
                <StatusBanner
                  severity="error"
                  message="Apply failed."
                  detail={applyError}
                  onDismiss={() => setApplyError(null)}
                />
              )}
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
                  <section
                    className="rounded-lg border border-danger/30 bg-danger-soft p-4"
                    aria-label="Settings recovery"
                  >
                    <h3 className="text-[14px] font-semibold">
                      Your settings file needs recovery{" "}
                      <HelpPopover title="Settings recovery">
                        An empty or malformed file cannot be merged safely. Recovery preserves the
                        original bytes first. Restoring a backup restores syntax, but does not prove
                        its endpoint or model works.
                      </HelpPopover>
                    </h3>
                    <p className="mt-2 text-[12px] text-ink-muted">{environment.settingsDetail}</p>
                    {environment.settingsState === "malformed_json" && (
                      <div className="mt-3 flex flex-wrap gap-2">
                        <button
                          type="button"
                          className="btn btn-secondary"
                          disabled={recovering}
                          onClick={() => setConfirmRecovery("backup")}
                        >
                          Restore valid backup…
                        </button>
                        <button
                          type="button"
                          className="btn btn-secondary"
                          disabled={recovering}
                          onClick={() => setConfirmRecovery("clean")}
                        >
                          Start clean…
                        </button>
                      </div>
                    )}
                    {confirmRecovery === "backup" && (
                      <RecoverySelector
                        backups={backupChoices}
                        error={backupChoiceError}
                        selected={selectedBackup}
                        onSelect={setSelectedBackup}
                      />
                    )}
                    {confirmRecovery && (
                      <div className="mt-3 text-[12px]">
                        <p>
                          {confirmRecovery === "backup"
                            ? `The selected backup ${selectedBackup ?? "(none selected)"} will replace settings. The invalid file is preserved separately.`
                            : "The invalid file will be preserved and replaced with an empty JSON object. You can then set up your providers."}
                        </p>
                        <div className="mt-2 flex gap-2">
                          <button
                            type="button"
                            className="btn btn-primary"
                            disabled={
                              recovering || (confirmRecovery === "backup" && !selectedBackup)
                            }
                            onClick={() => {
                              void handleRecovery(confirmRecovery === "backup");
                            }}
                          >
                            {recovering ? "Recovering…" : "Preserve original & recover"}
                          </button>
                          <button
                            type="button"
                            className="btn btn-secondary"
                            onClick={() => setConfirmRecovery(null)}
                          >
                            Cancel
                          </button>
                        </div>
                      </div>
                    )}
                    {restoreError && (
                      <p role="alert" className="mt-2 text-[12px] text-danger">
                        {restoreError}
                      </p>
                    )}
                  </section>
                )}

              {tab === "dashboard" && (
                <DashboardTab
                  configuredProvider={configuredProvider}
                  status={status}
                  statusError={statusError}
                  providers={providers}
                  selected={null}
                  credentials={savedCredentials}
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
                <>
                  <p className="text-[12px] text-ink-muted">
                    1. Choose a provider · 2. Add its keys · 3. Choose a model · 4. Review and
                    apply. Valid provider drafts stay on this device; API keys are never saved in
                    drafts.
                  </p>
                  {isCustom && customProviderDrafts().length > 0 && (
                    <FieldRow
                      label="Saved custom provider drafts"
                      htmlFor="custom-profile"
                      help="Reopens a local endpoint/model draft. It does not activate settings or move keys between providers."
                    >
                      <select
                        id="custom-profile"
                        className="h-9 w-full rounded border border-control bg-surface px-2"
                        value=""
                        onChange={(e) => {
                          const d = loadProviderDraft(e.target.value);
                          setCustomName(d.customName);
                          setCustomEndpoint(d.customEndpoint);
                          setSelectedModel(d.model);
                          setRoles(d.roles);
                          setContextWindow(d.context);
                          setResetRoles(d.resetRoles);
                          setResetContext(d.resetContext);
                          setApiKey("");
                          setConnectionTest(null);
                        }}
                      >
                        <option value="">Reopen a saved draft…</option>
                        {customProviderDrafts().map(({ id, draft }) => (
                          <option key={id} value={id}>
                            {draft.customName || new URL(draft.customEndpoint).hostname}
                          </option>
                        ))}
                      </select>
                    </FieldRow>
                  )}
                  <ProviderTab
                    hideKey={false}
                    providers={providers}
                    selectedId={selectedId}
                    onSelectProvider={(id) => {
                      if (id === selectedId) return;
                      setSelectedId(id);
                      if (id === EMPTY_ID) clearSessionString("provider");
                      else saveSessionString("provider", id);
                      const draft = loadProviderDraft(id);
                      setSelectedModel(draft.model);
                      setRoles(draft.roles);
                      setResetRoles(draft.resetRoles);
                      setResetContext(draft.resetContext);
                      setContextWindow(draft.context);
                      if (id === CUSTOM_ID) {
                        setCustomName(draft.customName);
                        setCustomEndpoint(draft.customEndpoint);
                      }
                      setFetchingModels(false);
                      setModels(null);
                      setModelsFetchedFor(null);
                      setModelsError(null);
                      // Invalidate any in-flight catalog/connection request and
                      // stale results for the previous provider.
                      fetchModelsSeq.current += 1;
                      testConnSeq.current += 1;
                      setTestingConnection(false);
                      // A typed key belongs to the provider it was typed for —
                      // carrying it into another provider's Apply would send
                      // that key to a different endpoint.
                      setApiKey("");
                      setConnectionTest(null);
                      setConnectionTestError(null);
                    }}
                    selected={selected}
                    isCustom={isCustom}
                    customName={customName}
                    onCustomName={setCustomName}
                    customEndpoint={customEndpoint}
                    onCustomEndpoint={(value) => {
                      setCustomEndpoint(value);
                      setApiKey("");
                      setSelectedModel(null);
                      setRoles(EMPTY_ROLES);
                      setResetRoles([]);
                      setContextWindow("");
                      setResetContext(false);
                      setConnectionTest(null);
                    }}
                    apiKey={apiKey}
                    onApiKey={(value) => {
                      setApiKey(value);
                      testConnSeq.current += 1;
                      setTestingConnection(false);
                      setConnectionTest(null);
                    }}
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
                </>
              )}

              {(tab === "credentials" ||
                (tab === "provider" && (selected !== null || isCustom))) && (
                <CredentialsTab
                  key={credentialProviderId ?? "none"}
                  onNavigateToProvider={() => navigate("provider")}
                  provider={selected}
                  isCustom={isCustom}
                  credentials={credentialList}
                  error={credentialsError}
                  addDisabled={credentialProviderId === null}
                  onAdd={handleAddCredential}
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

              {(tab === "model" || (tab === "provider" && (selected !== null || isCustom))) && (
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
                  onSelectModel={(id) => {
                    setSelectedModel(id);
                    // A newly chosen model starts with a blank declared
                    // window so the catalog suggestion can propose it fresh.
                    setContextWindow("");
                  }}
                  onContextWindowAdopt={(value) => {
                    setContextWindow(value);
                    setResetContext(false);
                  }}
                  modelInvalid={modelInvalid}
                  modelMessage={
                    modelValidation !== null && !modelValidation.ok ? modelValidation.message : null
                  }
                  status={status}
                  roles={roles}
                  resetRoles={resetRoles}
                  onRole={(key, value) => {
                    setRoles((r) => ({ ...r, [key]: value === "__reset__" ? "" : value }));
                    setResetRoles((current) =>
                      value === "__reset__"
                        ? [...new Set([...current, key])]
                        : current.filter((r) => r !== key),
                    );
                  }}
                  roleInvalid={roleInvalid}
                  advancedOpen={advancedOpen}
                  onToggleAdvanced={() => setAdvancedOpen((v) => !v)}
                  contextWindow={contextWindow}
                  onContextWindow={(value) => {
                    setContextWindow(value);
                    setResetContext(value === "__reset__");
                    if (value === "__reset__") setContextWindow("");
                  }}
                />
              )}

              {tab === "provider" && (selected !== null || isCustom) && (
                <section
                  aria-label="Review changes"
                  className="rounded-lg border border-line bg-surface p-4"
                >
                  <h3 className="text-[14px] font-semibold">
                    Review changes{" "}
                    <HelpPopover title="Review changes">
                      This compares your draft with saved global settings. Save validates and backs
                      up files; provider connectivity is a separate check. Stored keys remain
                      masked.
                    </HelpPopover>
                  </h3>
                  <dl className="mt-3 grid grid-cols-[auto_1fr] gap-x-4 gap-y-2 text-[12px]">
                    <dt>Saved provider</dt>
                    <dd>{configuredProvider?.name ?? "No recognized provider"}</dd>
                    <dt>Pending provider</dt>
                    <dd>{selected?.name ?? (customName || "Custom provider")}</dd>
                    <dt>Saved model</dt>
                    <dd className="break-all">
                      {status?.envModel ?? status?.model ?? "Claude Code default"}
                    </dd>
                    <dt>Pending model</dt>
                    <dd className="break-all">{selectedModel || "Keep saved model"}</dd>
                    <dt>Key</dt>
                    <dd>
                      {apiKey.trim()
                        ? "Replace with the key entered above (masked)"
                        : activeCredential
                          ? `Stored key · ••••${activeCredential.last4}`
                          : "No usable key selected"}
                    </dd>
                    <dt>Context window</dt>
                    <dd>
                      {resetContext ? "Remove override" : contextWindow || "Keep saved value"}
                    </dd>
                    <dt>Roles</dt>
                    <dd>
                      {Object.entries(roles)
                        .filter(([, v]) => v.trim())
                        .map(([k, v]) => `${k}: ${v}`)
                        .join(" · ") || "Keep saved values"}
                      {resetRoles.length > 0 ? ` · Remove: ${resetRoles.join(", ")}` : ""}
                    </dd>
                    <dt>Connection</dt>
                    <dd>
                      {selected?.id === "nvidia-nim"
                        ? "NVIDIA through the local helper; keys first, existing fallback providers retained"
                        : "Direct connection using one stored key"}
                    </dd>
                  </dl>
                  <p className="mt-3 text-[12px] text-ink-muted">
                    Blank model and role fields keep saved values. When changing providers, choose
                    compatible IDs or explicitly reset old overrides.
                  </p>
                </section>
              )}

              {tab === "provider" && resetContext && (
                <p role="status" className="text-[12px] text-warn">
                  Pending change: remove saved context-window override on Apply.
                </p>
              )}
              {tab === "provider" && resetRoles.length > 0 && (
                <p role="status" className="text-[12px] text-warn">
                  Pending role resets: {resetRoles.join(", ")}. Applied only when you save.
                </p>
              )}
              {tab === "provider" && (selected !== null || isCustom) && (
                <section className="rounded-lg border border-line bg-surface p-4">
                  <h3 className="text-[14px] font-semibold">
                    Use this provider for automatic switching{" "}
                    <HelpPopover title="Provider order">
                      This adds one provider group. Every enabled stored key is tried in key
                      priority order before moving to the next provider. Save the order, then enable
                      switching to activate routed mode.
                    </HelpPopover>
                  </h3>
                  <p className="mt-1 text-[12px] text-ink-muted">
                    Add keys above and choose a model, then add this provider to your switching
                    order.
                  </p>
                  <button
                    type="button"
                    className="btn btn-primary mt-3"
                    disabled={
                      !storedCredential ||
                      !selectedModel ||
                      modelInvalid ||
                      (selected?.protocol === "open_ai_only" && selected.id !== "nvidia-nim") ||
                      !routing
                    }
                    onClick={addProviderGroup}
                  >
                    Add provider to switching order
                  </button>
                </section>
              )}
              {tab === "routing" && (
                <>
                  <RoutingTab
                    ipc={ipc}
                    onAddProvider={() => navigate("provider")}
                    config={routing}
                    dirty={routing !== null && JSON.stringify(routing) !== savedRouting}
                    receipt={routingReceipt}
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
                    onChange={(cfg) => {
                      setRouting(cfg);
                      setRoutingReceipt(null);
                    }}
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
                  {tab === "routing" &&
                    ((selected?.protocol === "open_ai_only" && selected.id !== "nvidia-nim") ||
                      isCustom) && (
                      <BridgeCard
                        ipc={ipc}
                        providerId={selected === null ? null : selected.id}
                        providerName={selected?.name ?? (isCustom ? customName || "Custom" : null)}
                        model={selectedModel}
                        onApplied={(outcome) => {
                          setApplyOutcome(outcome);
                          // Refresh effective status after the apply transaction.
                          readStatus(ipc)
                            .then(setStatus)
                            .catch(() => {});
                        }}
                      />
                    )}
                </>
              )}

              {tab === "maintenance" && (
                <>
                  <RecoverySelector
                    backups={backupChoices}
                    error={backupChoiceError}
                    selected={selectedBackup}
                    onSelect={setSelectedBackup}
                  />
                  <MaintenanceTab
                    selectedBackup={selectedBackup}
                    backups={backups}
                    environment={environment}
                    environmentError={environmentError}
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
                </>
              )}
            </div>
          </div>

          {/* One primary action, always reachable, with the reason when it cannot run. */}
          {tab !== "routing" && tab !== "maintenance" && (
            <div className="border-t border-line bg-app px-6 py-3">
              <div className="mx-auto flex max-w-[52rem] items-center gap-3">
                {routedActive && selected?.id !== "nvidia-nim" ? (
                  confirmingDirect ? (
                    <>
                      <p className="mr-auto text-[12px] text-ink-muted">
                        Switch Claude Code back to a direct provider? The proxy keeps running, so
                        you can switch back at any time.
                      </p>
                      <button
                        type="button"
                        onClick={() => setConfirmingDirect(false)}
                        className="btn btn-secondary"
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
                        className="btn btn-secondary"
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
                    <p id="apply-disabled-reason" className="mr-auto text-[12px] text-ink-muted">
                      {applyHint}
                    </p>
                    <LoadingButton
                      testId="apply"
                      describedBy="apply-disabled-reason"
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
          )}
        </main>
      </div>
    </div>
  );
}
