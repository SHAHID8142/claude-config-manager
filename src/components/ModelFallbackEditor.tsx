import { useEffect, useId, useRef, useState } from "react";
import { ArrowDown, ArrowUp, Trash2 } from "lucide-react";
import {
  discoverModels,
  isCommandError,
  type IpcLike,
  type ModelEntry,
  type RouteTarget,
} from "../lib/ipc";
import { validateModelId } from "../lib/modelId";
import { FieldRow } from "./FieldRow";
import { HelpPopover } from "./HelpPopover";

export function ModelFallbackEditor({
  target,
  ipc,
  busy,
  onChange,
}: {
  target: RouteTarget;
  ipc: IpcLike;
  busy: boolean;
  onChange: (target: RouteTarget) => void;
}) {
  const id = useId();
  const [catalog, setCatalog] = useState<ModelEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [draft, setDraft] = useState("");
  const [error, setError] = useState<string | null>(null);
  const active = useRef(true);
  useEffect(() => {
    active.current = true;
    return () => {
      active.current = false;
    };
  }, []);
  const models = [target.model ?? "", ...(target.fallbackModels ?? [])];
  const catalogId = `${id}-catalog`;
  function update(next: string[]) {
    onChange({
      ...target,
      model: next[0]?.trim() || null,
      fallbackModels: next.slice(1).map((m) => m.trim()),
    });
  }
  function move(index: number, delta: number) {
    const next = [...models];
    [next[index], next[index + delta]] = [next[index + delta], next[index]];
    update(next);
    requestAnimationFrame(() =>
      document.getElementById(`${id}-move-${index + delta}-${delta}`)?.focus(),
    );
  }
  async function load() {
    setLoading(true);
    setError(null);
    try {
      const result = await discoverModels(target.providerId, undefined, ipc);
      if (active.current) setCatalog(result);
    } catch (err) {
      if (active.current)
        setError(
          isCommandError(err)
            ? err.message
            : "Models could not be loaded. You can still enter model IDs manually.",
        );
    } finally {
      if (active.current) setLoading(false);
    }
  }
  function add() {
    const check = validateModelId(draft);
    if (!check.ok) {
      setError(check.message);
      return;
    }
    if (!target.model?.trim()) {
      setError("Choose a primary model first.");
      return;
    }
    if (models.includes(check.value)) {
      setError("That model is already in this provider’s order.");
      return;
    }
    if (models.length >= 5) {
      setError("Use at most 5 models per provider.");
      return;
    }
    update([...models, check.value]);
    setDraft("");
    setError(null);
    document.getElementById(`${id}-add`)?.focus();
  }
  return (
    <fieldset disabled={busy} className="min-w-0 rounded-md border border-line p-3">
      <legend className="px-1 text-[13px] font-medium">
        Model order{" "}
        <HelpPopover title="Automatic model fallback">
          Try models top to bottom. For each model, try enabled keys in priority order, then the
          next model, then the next provider. A rejected key is paused across models; an unavailable
          model is paused across keys. Invalid requests stop. A response already streaming is never
          retried.
        </HelpPopover>
      </legend>
      <p id={`${id}-help`} className="mb-2 text-[12px] text-ink-muted">
        First model is preferred. Up to 5 models. Save the order below to apply your changes.
      </p>
      <ol aria-label="Model fallback order" className="space-y-2">
        {models.map((model, index) => (
          <li key={index} className="rounded-md bg-surface-sunken p-2">
            <FieldRow
              label={index === 0 ? "Primary model" : `Fallback model ${index}`}
              htmlFor={`${id}-model-${index}`}
              help="Choose a suggestion or enter an exact model ID supported by this provider. Catalog results do not guarantee model access or compatible tools."
            >
              <input
                id={`${id}-model-${index}`}
                name={`model-${index}`}
                list={catalogId}
                value={model}
                maxLength={256}
                autoCapitalize="none"
                autoCorrect="off"
                spellCheck={false}
                aria-describedby={`${id}-help`}
                className="h-9 w-full min-w-0 rounded-md border border-control bg-surface px-2 text-[13px]"
                onChange={(e) => {
                  const next = [...models];
                  next[index] = e.target.value;
                  update(next);
                  setError(null);
                }}
              />
            </FieldRow>
            <div className="flex gap-2">
              <button
                id={`${id}-move-${index}--1`}
                type="button"
                className="btn btn-secondary !h-8 !px-2"
                aria-label={`Move model ${index + 1} up`}
                disabled={index === 0}
                onClick={() => move(index, -1)}
              >
                <ArrowUp aria-hidden="true" size={14} />
              </button>
              <button
                id={`${id}-move-${index}-1`}
                type="button"
                className="btn btn-secondary !h-8 !px-2"
                aria-label={`Move model ${index + 1} down`}
                disabled={index === models.length - 1}
                onClick={() => move(index, 1)}
              >
                <ArrowDown aria-hidden="true" size={14} />
              </button>
              <button
                type="button"
                className="btn btn-secondary !h-8 !px-2"
                aria-label={`Remove model ${index + 1}`}
                disabled={models.length === 1}
                onClick={() => update(models.filter((_, i) => i !== index))}
              >
                <Trash2 aria-hidden="true" size={14} />
              </button>
            </div>
          </li>
        ))}
      </ol>
      <FieldRow
        label="Add fallback model"
        htmlFor={`${id}-add`}
        hint="Select a catalog suggestion or enter a model ID."
        help="Add another model from this same provider. Use the arrow buttons to set priority. Save order or Save & enable applies the list."
      >
        <input
          id={`${id}-add`}
          name="fallback-model"
          value={draft}
          list={catalogId}
          maxLength={256}
          autoCapitalize="none"
          autoCorrect="off"
          spellCheck={false}
          aria-invalid={error ? true : undefined}
          aria-describedby={error ? `${id}-error` : undefined}
          className="h-9 w-full rounded-md border border-control bg-surface px-2 text-[13px]"
          onChange={(e) => {
            setDraft(e.target.value);
            setError(null);
          }}
        />
      </FieldRow>
      <div className="flex flex-wrap gap-2">
        <button type="button" className="btn btn-secondary" onClick={add}>
          Add model
        </button>
        {!target.providerId.startsWith("custom") && (
          <button
            type="button"
            className="btn btn-secondary"
            disabled={loading}
            onClick={() => {
              void load();
            }}
          >
            {loading ? "Loading models…" : "Load provider models"}
          </button>
        )}
      </div>
      <datalist id={catalogId}>
        {catalog.map((m) => (
          <option key={m.id} value={m.id}>
            {m.displayName ?? m.id}
          </option>
        ))}
      </datalist>
      {catalog.length > 0 && (
        <p role="status" className="mt-2 text-[12px] text-ink-muted">
          {catalog.length} suggestions loaded. Manual IDs are still available.
        </p>
      )}
      {error && (
        <p id={`${id}-error`} role="alert" className="mt-2 text-[12px] text-danger">
          {error}
        </p>
      )}
      <p className="mt-3 break-all text-[12px] text-ink-muted">
        Model sequence: {models.map((m) => m || "Choose a model").join(" → ")}
      </p>
    </fieldset>
  );
}
