import { useEffect, useState } from "react";
import { ApiKeyField } from "./components/ApiKeyField";
import { BridgeBadge, CapabilityBadge } from "./components/CapabilityBadge";
import { FieldRow } from "./components/FieldRow";
import { LoadingButton } from "./components/LoadingButton";
import { ModelList } from "./components/ModelList";
import { Section } from "./components/Section";
import { StatusBanner } from "./components/StatusBanner";
import {
  checkEnvironment,
  defaultIpc,
  discoverModels,
  listProviders,
  readStatus,
  restoreLastBackup,
  saveAndApply,
  isCommandError,
  type ApplyOutcome,
  type EffectiveStatus,
  type EnvironmentReport,
  type IpcLike,
  type ModelEntry,
  type ProviderSummary,
  type RestoreOutcome,
  type StatusWarning,
} from "./lib/ipc";
import { classifyEndpoint, endpointCheckMessage } from "./lib/endpoint";
import { focusRing } from "./lib/focus";

const inputClass = `w-full rounded-md border border-zinc-300 bg-white px-3 py-2 text-sm text-zinc-700 placeholder-zinc-400 disabled:cursor-not-allowed disabled:opacity-50 dark:border-zinc-600 dark:bg-zinc-800 dark:text-zinc-200 ${focusRing}`;

const secondaryButtonClass = `rounded-md border border-zinc-300 px-3 py-1.5 text-sm font-medium text-zinc-700 disabled:cursor-not-allowed disabled:opacity-50 dark:border-zinc-600 dark:text-zinc-200 ${focusRing}`;

/** Sentinels for the provider selector: presets use their real ids;
 * custom entry and the empty state use dedicated sentinels. */
const CUSTOM_ID = "__custom__";
const EMPTY_ID = "__none__";

/** Honest mode label from the effective status (never fabricated). */
function modeLabel(status: EffectiveStatus | null): string {
  if (status === null) return "unknown";
  if (!status.settingsFound) return "not configured";
  if (status.apiKeyHelper !== null) return "direct (Keychain helper)";
  if (status.apiKeyPresent || status.authTokenPresent) return "direct (plaintext env)";
  return "not configured";
}

/** Warning text derived from the typed wire shape. */
function warningText(w: StatusWarning): string {
  if ("helperShadowedByEnv" in w) {
    return "apiKeyHelper is shadowed by a plaintext env credential — remove the env key to use the helper.";
  }
  if ("plaintextKeyInEnv" in w) {
    return `${w.plaintextKeyInEnv.which} is stored in plaintext inside settings.json.`;
  }
  if ("modelConflict" in w) {
    return `Model conflict: settings say "${w.modelConflict.settingModel}" but ANTHROPIC_MODEL says "${w.modelConflict.envModel}" (env wins).`;
  }
  return "Unknown warning";
}

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
  const [selectedId, setSelectedId] = useState<string>(EMPTY_ID);
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
  const [models, setModels] = useState<ModelEntry[] | null>(null);
  const [fetchingModels, setFetchingModels] = useState(false);
  const [modelsError, setModelsError] = useState<string | null>(null);
  const [modelsFetchedFor, setModelsFetchedFor] = useState<string | null>(null);
  const [selectedModel, setSelectedModel] = useState<string | null>(null);

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

  useEffect(() => {
    let cancelled = false;
    checkEnvironment(ipc)
      .then((env) => {
        if (!cancelled) setEnvironment(env);
      })
      .catch(() => {
        /* Environment probe is best-effort. */
      });
    return () => {
      cancelled = true;
    };
  }, [ipc]);

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

  const isCustom = selectedId === CUSTOM_ID;
  const selected = providers?.find((p) => p.id === selectedId) ?? null;

  const canApply =
    (selected !== null || (isCustom && customEndpoint.trim() !== "")) &&
    apiKey.trim() !== "" &&
    !applying;

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
    <main className="mx-auto flex min-h-screen max-w-2xl flex-col gap-4 bg-zinc-50 p-6 dark:bg-zinc-950">
      <header className="flex items-center justify-between">
        <h1 className="text-lg font-semibold text-zinc-900 dark:text-zinc-100">
          Claude Config Manager
        </h1>
        <span
          aria-label="Connection mode"
          className="rounded-full border border-zinc-300 px-2 py-0.5 text-xs text-zinc-500 dark:border-zinc-600 dark:text-zinc-400"
        >
          mode: {modeLabel(status)}
        </span>
      </header>

      {loadError !== null && (
        <StatusBanner
          severity="error"
          message="Could not load the provider catalog."
          detail={loadError}
        />
      )}

      {environment !== null && !environment.cliFound && (
        <StatusBanner
          severity="warning"
          message="Claude Code CLI not found on PATH."
          detail="Settings can still be managed here, but install the Claude Code CLI to use them."
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

      <Section title="Active configuration">
        {status === null ? (
          <p className="text-sm text-zinc-500 dark:text-zinc-400">Reading current configuration…</p>
        ) : status.settingsFound ? (
          <div className="flex flex-col gap-1 text-sm">
            <p className="text-zinc-700 dark:text-zinc-200">
              Model:{" "}
              <code className="rounded bg-zinc-100 px-1 dark:bg-zinc-800">
                {status.envModel ?? status.model ?? "not set"}
              </code>
              {status.model !== null &&
                status.envModel !== null &&
                status.model !== status.envModel && (
                  <span className="ml-1 text-xs text-amber-700 dark:text-amber-300">
                    (settings value overridden by ANTHROPIC_MODEL env)
                  </span>
                )}
            </p>
            {status.envBaseUrl !== null && (
              <p className="text-zinc-700 dark:text-zinc-200">
                Endpoint:{" "}
                <code className="rounded bg-zinc-100 px-1 dark:bg-zinc-800">
                  {status.envBaseUrl}
                </code>
              </p>
            )}
            <p className="text-zinc-700 dark:text-zinc-200">
              Credential:{" "}
              {status.apiKeyHelper !== null
                ? "Keychain via apiKeyHelper"
                : status.apiKeyPresent || status.authTokenPresent
                  ? "plaintext env (see warning)"
                  : "none"}
            </p>
            {status.warnings.length > 0 && (
              <ul className="mt-1 list-inside list-disc text-xs text-amber-700 dark:text-amber-300">
                {status.warnings.map((w, i) => (
                  <li key={i}>{warningText(w)}</li>
                ))}
              </ul>
            )}
          </div>
        ) : (
          <p className="text-sm text-zinc-500 dark:text-zinc-400">
            No settings file yet — the first Save &amp; Apply creates it.
          </p>
        )}
      </Section>

      <Section title="Provider">
        <FieldRow
          label="Provider"
          htmlFor="provider-select"
          hint={
            providers === null
              ? "Loading provider catalog…"
              : "Custom providers use your own Anthropic-compatible endpoint."
          }
        >
          <select
            id="provider-select"
            value={selectedId}
            disabled={providers === null}
            onChange={(e) => {
              setSelectedId(e.target.value);
              // A model id from another provider must never carry over.
              setSelectedModel(null);
              setModels(null);
              setModelsFetchedFor(null);
              setModelsError(null);
            }}
            className={inputClass}
          >
            <option value={EMPTY_ID}>
              {providers === null ? "Loading…" : "Select a provider"}
            </option>
            {providers?.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
            {providers !== null && <option value={CUSTOM_ID}>Custom…</option>}
          </select>
        </FieldRow>

        {selected !== null && (
          <div className="mb-2 flex flex-wrap items-center gap-2">
            <CapabilityBadge
              status={selected.capabilityStatus.status}
              sourceUrl={
                selected.capabilityStatus.status === "verified"
                  ? selected.capabilityStatus.source_url
                  : null
              }
            />
            <BridgeBadge required={selected.bridge === "required_for_open_ai_only"} />
          </div>
        )}

        {isCustom && (
          <>
            <FieldRow
              label="Custom provider name"
              htmlFor="custom-name-input"
              hint="Shown in the UI only."
            >
              <input
                id="custom-name-input"
                type="text"
                value={customName}
                onChange={(e) => setCustomName(e.target.value)}
                placeholder="My gateway"
                className={inputClass}
              />
            </FieldRow>
            <FieldRow
              label="Custom endpoint URL"
              htmlFor="custom-endpoint-input"
              hint="Must be an Anthropic-compatible https endpoint."
            >
              <input
                id="custom-endpoint-input"
                type="url"
                value={customEndpoint}
                onChange={(e) => setCustomEndpoint(e.target.value)}
                placeholder="https://"
                className={inputClass}
              />
            </FieldRow>
            {customEndpoint !== "" &&
              (() => {
                const check = classifyEndpoint(customEndpoint);
                const msg = endpointCheckMessage(check);
                if (msg === null) return null;
                const isError = check.check === "invalid" || check.check === "insecure";
                return (
                  <p
                    aria-live="polite"
                    id="custom-endpoint-check"
                    className={`mb-2 text-xs ${isError ? "font-medium text-red-600 dark:text-red-400" : "text-zinc-500 dark:text-zinc-400"}`}
                  >
                    {msg}
                  </p>
                );
              })()}
          </>
        )}

        {selected !== null && !isCustom && selected.protocol !== "open_ai_only" && (
          <FieldRow
            label="Endpoint URL"
            htmlFor="endpoint-input"
            hint="Provided by the verified preset and not editable."
          >
            <input
              id="endpoint-input"
              type="url"
              disabled
              value={selected.protocol === "anthropic_native" ? "https://api.anthropic.com" : ""}
              readOnly
              className={inputClass}
            />
          </FieldRow>
        )}

        <FieldRow
          label="API key"
          htmlFor="api-key-input"
          hint={
            selected === null && !isCustom
              ? "Enabled once a provider is selected."
              : "Stored in the macOS Keychain when saving. Never written to logs or backups in plaintext."
          }
        >
          <ApiKeyField
            id="api-key-input"
            value={apiKey}
            onChange={setApiKey}
            disabled={selected === null && !isCustom}
          />
        </FieldRow>
      </Section>

      <Section title="Model">
        {modelsError !== null && (
          <StatusBanner
            severity="error"
            message="Model discovery failed."
            detail={modelsError}
            onDismiss={() => setModelsError(null)}
          />
        )}
        <div className="mb-2 flex items-center gap-2">
          <LoadingButton
            label="Fetch models"
            pendingLabel="Fetching…"
            pending={fetchingModels}
            disabled={selected === null || isCustom}
            variant="secondary"
            onClick={() => {
              void handleFetchModels();
            }}
          />
          {selected !== null && !isCustom && (
            <span className="text-xs text-zinc-500 dark:text-zinc-400">
              Catalog source: verified preset for {selected.name}
            </span>
          )}
        </div>
        {modelsFetchedFor !== null &&
          (models !== null && models.length > 0 ? (
            <div className="mb-2">
              <ModelList
                models={models}
                selectedId={selectedModel}
                onSelect={(id) => setSelectedModel(id)}
              />
            </div>
          ) : (
            <p className="text-xs text-zinc-500 dark:text-zinc-400">
              Catalog returned no models — use manual Model ID entry below.
            </p>
          ))}
        <FieldRow
          label="Model"
          htmlFor="model-input"
          hint="Selected from the catalog or entered manually; never auto-selected."
        >
          <input
            id="model-input"
            type="text"
            value={selectedModel ?? ""}
            readOnly={models !== null && models.length > 0}
            disabled={selected === null && !isCustom}
            onChange={(e) => setSelectedModel(e.target.value)}
            placeholder="provider-model-id"
            className={inputClass}
          />
        </FieldRow>
      </Section>

      <Section title="Actions">
        {applyOutcome !== null && (
          <StatusBanner
            severity="success"
            message="Configuration applied and verified."
            detail={`Backup: ${applyOutcome.backupFileName || "(none — first run)"} · helper: ${applyOutcome.helperPath}`}
            onDismiss={() => setApplyOutcome(null)}
          />
        )}
        {applyError !== null && (
          <StatusBanner
            severity="error"
            message="Save & Apply failed."
            detail={applyError}
            onDismiss={() => setApplyError(null)}
          />
        )}
        {restoreOutcome !== null && (
          <StatusBanner
            severity="success"
            message="Settings restored from the latest backup."
            detail={`Restored: ${restoreOutcome.restoredFrom} · pre-restore copy: ${restoreOutcome.preRestoreBackup || "(none — file was missing)"}`}
            onDismiss={() => setRestoreOutcome(null)}
          />
        )}
        {restoreError !== null && (
          <StatusBanner
            severity="error"
            message="Restore failed."
            detail={restoreError}
            onDismiss={() => setRestoreError(null)}
          />
        )}
        <div className="flex flex-wrap gap-2">
          <button type="button" disabled className={secondaryButtonClass}>
            Test Connection
          </button>
          <LoadingButton
            label="Save & Apply"
            pendingLabel="Applying…"
            pending={applying}
            disabled={(selected === null && !isCustom) || apiKey.trim() === ""}
            variant="primary"
            onClick={() => {
              void handleApply();
            }}
          />
          <LoadingButton
            label="Restore last backup"
            pendingLabel="Restoring…"
            pending={restoring}
            onClick={() => {
              void handleRestore();
            }}
          />
        </div>
        {selected !== null && (
          <p className="mt-2 text-xs text-zinc-500 dark:text-zinc-400">
            The API key is stored in the macOS Keychain and handed to Claude Code via an
            apiKeyHelper script — it is never written into settings.json.
          </p>
        )}
      </Section>

      <details className="rounded-lg border border-zinc-200 bg-white p-4 dark:border-zinc-700 dark:bg-zinc-900">
        <summary
          className={`cursor-pointer rounded-md text-sm font-semibold text-zinc-700 dark:text-zinc-200 ${focusRing}`}
        >
          Advanced
        </summary>
        <p className="mt-2 text-sm text-zinc-500 dark:text-zinc-400">
          Advanced overrides arrive in a later task and default to OFF.
        </p>
      </details>
    </main>
  );
}
