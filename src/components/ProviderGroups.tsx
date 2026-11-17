import { useCallback, useEffect, useState } from "react";
import { ArrowDown, ArrowUp, Trash2 } from "lucide-react";
import { CredentialsPanel } from "./CredentialsPanel";
import { FieldRow } from "./FieldRow";
import { HelpPopover } from "./HelpPopover";
import {
  addCredential,
  listCredentials,
  removeCredential,
  promoteCredential,
  setCredentialEnabled,
  isCommandError,
  type CredentialView,
  type IpcLike,
  type ProviderSummary,
  type RoutingConfig,
  type RouteTarget,
} from "../lib/ipc";

interface Props {
  config: RoutingConfig;
  providers: ProviderSummary[];
  ipc: IpcLike;
  onChange: (config: RoutingConfig) => void;
  busy: boolean;
}
function ProviderGroup({
  target,
  name,
  ipc,
  onChange,
}: {
  target: RouteTarget;
  name: string;
  ipc: IpcLike;
  onChange: (target: RouteTarget) => void;
}) {
  const [keys, setKeys] = useState<CredentialView[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const reload = useCallback(async () => {
    try {
      setKeys(await listCredentials(target.providerId, ipc));
      setError(null);
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Stored keys could not be read.");
    }
  }, [target.providerId, ipc]);
  useEffect(() => {
    let cancelled = false;
    listCredentials(target.providerId, ipc)
      .then((list) => {
        if (!cancelled) {
          setKeys(list);
          setError(null);
        }
      })
      .catch((err: unknown) => {
        if (!cancelled)
          setError(isCommandError(err) ? err.message : "Stored keys could not be read.");
      });
    return () => {
      cancelled = true;
    };
  }, [target.providerId, ipc]);
  async function mutate(action: () => Promise<unknown>): Promise<boolean> {
    try {
      await action();
      await reload();
      return true;
    } catch (err) {
      setError(isCommandError(err) ? err.message : "The key could not be updated.");
      return false;
    }
  }
  return (
    <div className="mt-3 flex flex-col gap-3">
      {error && (
        <div role="alert" className="text-[12px] text-danger">
          {error}{" "}
          <button
            type="button"
            className="underline"
            onClick={() => {
              void reload();
            }}
          >
            Retry reading keys
          </button>
        </div>
      )}
      <p className="text-[12px] text-ink-muted">
        {keys === null
          ? "Reading stored keys…"
          : target.credentialId
            ? "Existing route: uses its explicitly selected key. Convert to all keys to use this provider’s priority order."
            : `${keys.filter((k) => k.enabled).length} enabled keys · tried top to bottom before the next provider`}
      </p>
      {target.credentialId && (
        <button
          type="button"
          className="btn btn-secondary self-start"
          onClick={() => onChange({ ...target, credentialId: null })}
        >
          Use all enabled keys for {name}
        </button>
      )}
      <div className="grid gap-3 sm:grid-cols-2">
        <FieldRow
          label="Model"
          htmlFor={`model-${target.id}`}
          hint="Required model mapping for this provider."
        >
          <input
            id={`model-${target.id}`}
            className="h-9 w-full rounded-md border border-control bg-surface px-2 text-[13px]"
            value={target.model ?? ""}
            onChange={(e) => onChange({ ...target, model: e.target.value.trim() || null })}
            spellCheck={false}
          />
        </FieldRow>
        <FieldRow
          label="Endpoint URL"
          htmlFor={`endpoint-${target.id}`}
          help="This provider's base URL. Check the request URL below. API keys are sent only to this destination."
        >
          <input
            id={`endpoint-${target.id}`}
            className="h-9 w-full rounded-md border border-control bg-surface px-2 text-[13px]"
            value={target.baseUrl}
            onChange={(e) => onChange({ ...target, baseUrl: e.target.value })}
            spellCheck={false}
          />
        </FieldRow>
      </div>
      <p className="break-all font-mono text-[11px] text-ink-subtle">
        Requests go to: {target.baseUrl.replace(/\/+$/, "")}
        {target.providerId === "nvidia-nim"
          ? target.baseUrl.replace(/\/+$/, "").endsWith("/v1")
            ? "/chat/completions"
            : "/v1/chat/completions"
          : target.baseUrl.replace(/\/+$/, "").endsWith("/v1")
            ? "/messages"
            : "/v1/messages"}
      </p>
      <details className="rounded-md border border-line p-3" open={keys?.length === 0 || undefined}>
        <summary className="cursor-pointer text-[12px] font-medium">
          Keys for {name}{" "}
          <span className="font-normal text-ink-muted">· {keys?.length ?? "…"} stored</span>
        </summary>
        <div className="mt-3">
          <CredentialsPanel
            credentials={keys ?? []}
            error={error}
            addDisabled={keys === null}
            onAdd={(label, secret) =>
              mutate(() => addCredential(target.providerId, label, secret, ipc))
            }
            onRemove={(id) => {
              void mutate(() => removeCredential(id, ipc));
            }}
            onPromote={(id) => {
              void mutate(() => promoteCredential(id, ipc));
            }}
            onToggle={(id, enabled) => {
              void mutate(() => setCredentialEnabled(id, enabled, ipc));
            }}
          />
        </div>
      </details>
    </div>
  );
}
export function ProviderGroups({ config, providers, ipc, onChange, busy }: Props) {
  function move(index: number, delta: number) {
    const targets = [...config.targets];
    [targets[index], targets[index + delta]] = [targets[index + delta], targets[index]];
    onChange({ ...config, targets });
  }
  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center gap-1 text-[13px] font-medium">
        Provider priority{" "}
        <HelpPopover title="Provider and key order">
          Try all enabled keys in provider 1, in their key order, then provider 2. A healthy first
          key stays in use. Temporary failures pause keys; keys from one account may share a limit.
          Changes to provider/model order remain pending until you save.
        </HelpPopover>
      </div>
      {config.targets.length === 0 && (
        <p className="rounded-lg border border-dashed border-line p-6 text-center text-[13px] text-ink-muted">
          Add your first provider, store its keys, and choose a model to build your switching order.
        </p>
      )}
      {config.targets.map((target, index) => {
        const name =
          providers.find((p) => p.id === target.providerId)?.name ??
          `Custom gateway (${target.id})`;
        return (
          <section
            key={target.id}
            aria-label={`${name} priority ${index + 1}`}
            className="rounded-lg border border-line bg-surface p-4"
          >
            <div className="flex flex-wrap items-center gap-2">
              <span className="grid size-7 place-items-center rounded-full bg-accent-soft text-[12px] font-semibold text-accent">
                {index + 1}
              </span>
              <h3 className="mr-auto text-[14px] font-semibold">{name}</h3>
              <label className="flex items-center gap-1.5 text-[12px]">
                <input
                  type="checkbox"
                  checked={target.enabled}
                  disabled={busy}
                  onChange={(e) =>
                    onChange({
                      ...config,
                      targets: config.targets.map((t) =>
                        t.id === target.id ? { ...t, enabled: e.target.checked } : t,
                      ),
                    })
                  }
                />
                Enabled
              </label>
              <button
                type="button"
                className="btn btn-secondary !size-8 !p-0"
                aria-label={`Move ${name} up`}
                disabled={busy || index === 0}
                onClick={() => move(index, -1)}
              >
                <ArrowUp size={14} />
              </button>
              <button
                type="button"
                className="btn btn-secondary !size-8 !p-0"
                aria-label={`Move ${name} down`}
                disabled={busy || index === config.targets.length - 1}
                onClick={() => move(index, 1)}
              >
                <ArrowDown size={14} />
              </button>
              <button
                type="button"
                className="btn btn-secondary !size-8 !p-0"
                aria-label={`Remove ${name} from switching`}
                disabled={busy}
                onClick={() =>
                  onChange({ ...config, targets: config.targets.filter((t) => t.id !== target.id) })
                }
              >
                <Trash2 size={14} />
              </button>
            </div>
            <ProviderGroup
              target={target}
              name={name}
              ipc={ipc}
              onChange={(next) =>
                onChange({
                  ...config,
                  targets: config.targets.map((t) => (t.id === target.id ? next : t)),
                })
              }
            />
          </section>
        );
      })}
    </div>
  );
}
