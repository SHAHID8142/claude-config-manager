/** V2-21: credential management with masked display.
 *
 * The UI never receives a key back from the backend — only `last4`. Masked rows
 * show `•••• 1f2a`, so a screenshot, a shared screen, or a support transcript
 * cannot leak a credential. A migrated pre-V2 key is labelled and cannot be
 * deleted from here (it is the file Claude Code already reads). */
import { useId, useState } from "react";
import { ArrowUp, KeyRound, Trash2 } from "lucide-react";
import type { CredentialView } from "../lib/ipc";
import { maskKey } from "../lib/maskKey";
import { FieldRow } from "./FieldRow";
import { HelpPopover } from "./HelpPopover";

export interface CredentialsPanelProps {
  credentials: CredentialView[];
  /** Adding is disabled until a provider is selected. */
  addDisabled?: boolean;
  onAdd: (label: string, secret: string) => void | Promise<boolean | void>;
  onRemove: (id: string) => void;
  onToggle: (id: string, enabled: boolean) => void;
  onPromote: (id: string) => void;
  /** Shown under the list when the backend refused something. */
  error?: string | null;
}

export function CredentialsPanel({
  credentials,
  addDisabled = false,
  onAdd,
  onRemove,
  onToggle,
  onPromote,
  error = null,
}: CredentialsPanelProps) {
  const fieldId = useId();
  const labelId = `${fieldId}-label`;
  const secretId = `${fieldId}-secret`;
  const [label, setLabel] = useState("");
  const [secret, setSecret] = useState("");

  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  async function submit() {
    if (secret.trim() === "") return;
    if (saving || addDisabled) return;
    setSaving(true);
    setSaveError(null);
    try {
      const saved = await onAdd(label.trim() === "" ? "key" : label.trim(), secret.trim());
      if (saved !== false) {
        setLabel("");
        setSecret("");
      }
    } catch {
      setSaveError(
        "The key could not be saved. Your entry has been kept; retry when the problem is resolved.",
      );
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="flex flex-col gap-2">
      {credentials.length === 0 ? (
        <p className="text-[12px] text-ink-subtle">
          No key stored for this provider yet. Add one below, or type it in the API key field.
        </p>
      ) : (
        <ul aria-label="Stored credentials" className="flex flex-col gap-1">
          {credentials.map((c) => (
            <li
              key={c.id}
              className="flex items-center gap-2 rounded-md border border-control bg-surface px-2 py-1.5"
            >
              <KeyRound aria-hidden="true" className="size-3.5 shrink-0 text-ink-subtle" />
              <span className="min-w-0 flex-1 truncate text-[12.5px] text-ink">{c.label}</span>
              <span className="shrink-0 font-mono text-[11.5px] text-ink-muted">
                {maskKey(c.last4)}
              </span>
              {c.legacy && (
                <span className="shrink-0 rounded border border-line px-1 text-[10px] text-ink-subtle">
                  existing
                </span>
              )}
              {c.priority > 0 && (
                <button
                  type="button"
                  onClick={() => onPromote(c.id)}
                  aria-label={`Try ${c.label} first`}
                  title="Move this key to the top of this provider’s order"
                  className="inline-grid min-h-6 min-w-6 shrink-0 place-items-center rounded p-1 text-ink-subtle hover:text-ink"
                >
                  <ArrowUp aria-hidden="true" className="size-3.5" />
                </button>
              )}
              {!c.legacy && (
                <button
                  type="button"
                  onClick={() => onRemove(c.id)}
                  aria-label={`Delete ${c.label}`}
                  className="inline-grid min-h-6 min-w-6 shrink-0 place-items-center rounded p-1 text-ink-subtle hover:text-danger"
                >
                  <Trash2 aria-hidden="true" className="size-3.5" />
                </button>
              )}
              <button
                type="button"
                role="switch"
                aria-checked={c.enabled}
                aria-label={`Enable ${c.label}`}
                onClick={() => onToggle(c.id, !c.enabled)}
                className={`relative h-4 w-7 shrink-0 rounded-full border transition-colors ${
                  c.enabled ? "border-ok/40 bg-ok/25" : "border-line bg-surface-sunken"
                }`}
              >
                <span
                  aria-hidden="true"
                  className={`absolute top-0.5 size-2.5 rounded-full transition-all ${
                    c.enabled ? "left-3.5 bg-ok" : "left-0.5 bg-ink-subtle"
                  }`}
                />
              </button>
            </li>
          ))}
        </ul>
      )}

      <div className="flex items-center gap-1 text-[12px] font-medium text-ink">
        Add a key{" "}
        <HelpPopover title="Add a key">
          Keys are tried in their displayed order within this provider. Give each key a label.
          Saving alone does not activate automatic switching. Keys on the same account may share
          limits.
        </HelpPopover>
      </div>
      <form
        onSubmit={(event) => {
          event.preventDefault();
          void submit();
        }}
        className="flex flex-col gap-2 rounded-md border border-dashed border-line p-3"
      >
        <div className="grid gap-2 sm:grid-cols-[minmax(0,1fr)_minmax(0,1.5fr)]">
          <FieldRow
            label="Key label"
            htmlFor={labelId}
            help="A name such as Work or Personal. This label identifies the key in the order and activity view; never paste your secret here."
          >
            <input
              id={labelId}
              value={label}
              onChange={(event) => setLabel(event.target.value)}
              placeholder="e.g. Work"
              disabled={saving || addDisabled}
              className="h-9 min-w-0 w-full rounded border border-control bg-surface px-2 text-[12px] text-ink"
            />
          </FieldRow>
          <FieldRow
            label="New API key"
            htmlFor={secretId}
            help="Paste a credential for this provider. It remains masked and is stored privately. A successful save clears this field; a failed save keeps it for retry."
          >
            <input
              id={secretId}
              type="password"
              value={secret}
              onChange={(event) => setSecret(event.target.value)}
              placeholder="Paste key"
              autoComplete="off"
              spellCheck={false}
              disabled={saving || addDisabled}
              className="h-9 min-w-0 w-full rounded border border-control bg-surface px-2 font-mono text-[12px] text-ink"
            />
          </FieldRow>
        </div>
        <button
          type="submit"
          disabled={saving || addDisabled || secret.trim() === ""}
          className="btn btn-secondary self-start"
        >
          {saving ? "Saving…" : "Add key"}
        </button>
        <p className="text-[11px] text-ink-subtle">
          Saved privately on this Mac. Only the last four characters are shown after saving.
        </p>
      </form>

      {saveError && (
        <p role="alert" className="text-[12px] text-danger">
          {saveError}
        </p>
      )}
      {error !== null && (
        <p role="alert" className="text-[11.5px] text-danger">
          {error}
        </p>
      )}
    </div>
  );
}
