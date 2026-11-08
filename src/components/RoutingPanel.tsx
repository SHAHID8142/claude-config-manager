/** V2-23/24/30/31: the failover routing chain and the loopback proxy.
 *
 * The panel is metadata-only: targets name a provider, a stored credential id,
 * a model and an endpoint — never a key. Ordering is explicit (up/down buttons
 * plus position numbers) because the proxy tries targets top to bottom.
 */
import { ArrowDown, ArrowUp, Plus, ShieldCheck, Trash2 } from "lucide-react";
import { useState } from "react";
import type {
  CredentialView,
  ProviderSummary,
  ProxyStatus,
  RouteTarget,
  RoutingConfig,
} from "../lib/ipc";

export interface RoutingPanelProps {
  config: RoutingConfig;
  providers: ProviderSummary[];
  credentials: CredentialView[];
  status: ProxyStatus | null;
  busy?: boolean;
  onChange: (next: RoutingConfig) => void;
  onSave: () => void;
  onStartProxy: () => void;
  onStopProxy: () => void;
  onApplyRouted: () => void;
  /** Ask the app to load credentials for the provider the owner just picked. */
  onNeedCredentials?: (providerId: string) => void;
  error?: string | null;
}

const inputClass =
  "h-8 w-full rounded-md border border-line bg-surface px-2 text-[12.5px] text-ink placeholder:text-ink-subtle focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent focus-visible:ring-offset-2 focus-visible:ring-offset-app";

function reorder(targets: RouteTarget[], from: number, to: number): RouteTarget[] {
  if (to < 0 || to >= targets.length) return targets;
  const next = targets.slice();
  const [moved] = next.splice(from, 1);
  next.splice(to, 0, moved);
  return next;
}

export function RoutingPanel({
  config,
  providers,
  credentials,
  status,
  busy = false,
  onChange,
  onSave,
  onStartProxy,
  onStopProxy,
  onApplyRouted,
  onNeedCredentials,
  error = null,
}: RoutingPanelProps) {
  const [draftProvider, setDraftProvider] = useState("");
  const [draftCredential, setDraftCredential] = useState("");
  const [draftModel, setDraftModel] = useState("");

  const compatProviders = providers.filter(
    (p) => p.directBaseUrl !== null && p.protocol !== "open_ai_only",
  );
  const provider = compatProviders.find((p) => p.id === draftProvider) ?? null;
  const providerCredentials = credentials.filter(
    (c) => c.providerId === (provider?.id ?? "") && c.enabled,
  );

  const canAdd =
    provider !== null && draftCredential !== "" && config.targets.length < 10;

  function addTarget() {
    if (!provider || !canAdd) return;
    const target: RouteTarget = {
      id: `${provider.id}-${config.targets.length + 1}`,
      providerId: provider.id,
      credentialId: draftCredential,
      model: draftModel.trim() === "" ? null : draftModel.trim(),
      baseUrl: provider.directBaseUrl ?? "",
      enabled: true,
    };
    onChange({ ...config, targets: [...config.targets, target] });
    setDraftModel("");
  }

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center justify-between gap-3">
        <label className="flex items-center gap-2 text-[12.5px] text-ink" htmlFor="routing-on">
          <input
            id="routing-on"
            type="checkbox"
            className="size-3.5 accent-[var(--accent)]"
            checked={config.enabled}
            onChange={(e) => onChange({ ...config, enabled: e.target.checked })}
          />
          Fail over between targets
        </label>
        <button
          type="button"
          className="h-7 rounded-md border border-line px-2 text-[12px] text-ink hover:bg-surface-sunken disabled:opacity-50"
          onClick={onSave}
          disabled={busy}
        >
          Save chain
        </button>
      </div>

      <p className="text-[11.5px] text-ink-subtle">
        Tried top to bottom. A 429, 5xx, timeout or rejected key moves to the next target; a
        malformed request does not, so nothing is charged twice.
      </p>

      {config.targets.length === 0 ? (
        <p className="text-[12px] text-ink-subtle">No targets yet. Add one below.</p>
      ) : (
        <ol aria-label="Routing targets in priority order" className="flex flex-col gap-1">
          {config.targets.map((t, i) => (
            <li
              key={t.id}
              className="flex items-center gap-2 rounded-md border border-line bg-surface-raised px-2 py-1.5"
            >
              <span className="w-4 shrink-0 text-center text-[11px] tabular-nums text-ink-subtle">
                {i + 1}
              </span>
              <span className="min-w-0 flex-1 truncate text-[12.5px] text-ink">
                {providers.find((p) => p.id === t.providerId)?.name ?? t.providerId}
                <span className="text-ink-subtle"> · {t.model ?? "provider default model"}</span>
              </span>
              {status?.cooling.includes(t.id) ? (
                <span className="shrink-0 rounded-sm bg-warn-soft px-1.5 py-0.5 text-[10.5px] text-warn">
                  cooling down
                </span>
              ) : null}
              <button
                type="button"
                aria-label={`Move ${t.id} up`}
                className="rounded-sm p-1 text-ink-muted hover:bg-surface-sunken hover:text-ink disabled:opacity-40"
                disabled={i === 0}
                onClick={() => onChange({ ...config, targets: reorder(config.targets, i, i - 1) })}
              >
                <ArrowUp aria-hidden="true" className="size-3.5" />
              </button>
              <button
                type="button"
                aria-label={`Move ${t.id} down`}
                className="rounded-sm p-1 text-ink-muted hover:bg-surface-sunken hover:text-ink disabled:opacity-40"
                disabled={i === config.targets.length - 1}
                onClick={() => onChange({ ...config, targets: reorder(config.targets, i, i + 1) })}
              >
                <ArrowDown aria-hidden="true" className="size-3.5" />
              </button>
              <button
                type="button"
                aria-label={`Remove ${t.id}`}
                className="rounded-sm p-1 text-ink-muted hover:bg-danger-soft hover:text-danger"
                onClick={() =>
                  onChange({
                    ...config,
                    targets: config.targets.filter((_, idx) => idx !== i),
                  })
                }
              >
                <Trash2 aria-hidden="true" className="size-3.5" />
              </button>
            </li>
          ))}
        </ol>
      )}

      <div className="grid grid-cols-2 gap-2">
        <label className="flex flex-col gap-1 text-[11.5px] text-ink-muted">
          Provider
          <select
            className={inputClass}
            value={draftProvider}
            onChange={(e) => {
              setDraftProvider(e.target.value);
              setDraftCredential("");
              if (e.target.value !== "") onNeedCredentials?.(e.target.value);
            }}
          >
            <option value="">Choose…</option>
            {compatProviders.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
          </select>
        </label>
        <label className="flex flex-col gap-1 text-[11.5px] text-ink-muted">
          Credential
          <select
            className={inputClass}
            value={draftCredential}
            onChange={(e) => setDraftCredential(e.target.value)}
            disabled={providerCredentials.length === 0}
          >
            <option value="">
              {providerCredentials.length === 0 ? "No stored key" : "Choose…"}
            </option>
            {providerCredentials.map((c) => (
              <option key={c.id} value={c.id}>
                {c.label}
              </option>
            ))}
          </select>
        </label>
        <label className="col-span-2 flex flex-col gap-1 text-[11.5px] text-ink-muted">
          Model for this target (blank keeps the caller's model)
          <input
            className={inputClass}
            value={draftModel}
            placeholder="nvidia/nemotron-3-ultra-550b-a55b:free"
            onChange={(e) => setDraftModel(e.target.value)}
          />
        </label>
      </div>
      <button
        type="button"
        className="flex h-8 items-center justify-center gap-1.5 rounded-md border border-line text-[12px] text-ink hover:bg-surface-sunken disabled:opacity-50"
        onClick={addTarget}
        disabled={!canAdd || busy}
      >
        <Plus aria-hidden="true" className="size-3.5" />
        Add target
      </button>

      <div className="mt-1 flex flex-col gap-2 rounded-md border border-line bg-surface-sunken p-2">
        <div className="flex items-center justify-between gap-2">
          <span className="flex items-center gap-1.5 text-[12px] text-ink">
            <ShieldCheck aria-hidden="true" className="size-3.5 text-ink-subtle" />
            Loopback proxy
          </span>
          <span className="text-[11.5px] text-ink-muted">
            {status?.running ? `running on 127.0.0.1:${status.port ?? "…"}` : "stopped"}
          </span>
        </div>
        <div className="flex gap-2">
          <button
            type="button"
            className="h-8 flex-1 rounded-md border border-line bg-surface text-[12px] text-ink hover:bg-surface-sunken disabled:opacity-50"
            onClick={onStartProxy}
            disabled={busy || status?.running === true}
          >
            Start proxy
          </button>
          <button
            type="button"
            className="h-8 flex-1 rounded-md border border-line bg-surface text-[12px] text-ink hover:bg-surface-sunken disabled:opacity-50"
            onClick={onStopProxy}
            disabled={busy || !status?.running}
          >
            Stop
          </button>
          <button
            type="button"
            className="h-8 flex-1 rounded-md bg-accent text-[12px] text-ink-on-accent hover:bg-accent-hover disabled:opacity-50"
            onClick={onApplyRouted}
            disabled={busy || !config.enabled || config.targets.length === 0}
          >
            Route Claude Code
          </button>
        </div>
        {status ? (
          <p className="text-[11.5px] text-ink-muted">
            {status.requests} requests · {status.fallbacks} fallbacks · {status.failures} failed
            {status.lastTarget ? ` · last answer: ${status.lastTarget}` : ""}
          </p>
        ) : null}
        {status?.lastError ? (
          <p className="break-words text-[11.5px] text-danger">{status.lastError}</p>
        ) : null}
      </div>

      {error ? <p className="break-words text-[11.5px] text-danger">{error}</p> : null}
    </div>
  );
}
