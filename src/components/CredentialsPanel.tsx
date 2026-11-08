/** V2-21: credential management with masked display.
 *
 * The UI never receives a key back from the backend — only `last4`. Masked rows
 * show `•••• 1f2a`, so a screenshot, a shared screen, or a support transcript
 * cannot leak a credential. A migrated pre-V2 key is labelled and cannot be
 * deleted from here (it is the file Claude Code already reads). */
import { useState } from "react";
import { ArrowUp, KeyRound, Trash2 } from "lucide-react";
import type { CredentialView } from "../lib/ipc";
import { maskKey } from "../lib/maskKey";

export interface CredentialsPanelProps {
  credentials: CredentialView[];
  /** Adding is disabled until a provider is selected. */
  addDisabled?: boolean;
  onAdd: (label: string, secret: string) => void;
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
  const [label, setLabel] = useState("");
  const [secret, setSecret] = useState("");

  function submit() {
    if (secret.trim() === "") return;
    onAdd(label.trim() === "" ? "key" : label.trim(), secret.trim());
    setLabel("");
    setSecret("");
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
              className="flex items-center gap-2 rounded-md border border-line bg-surface-raised px-2 py-1.5"
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
                  title="Move to the front of the failover order"
                  className="shrink-0 rounded p-0.5 text-ink-subtle hover:text-ink"
                >
                  <ArrowUp aria-hidden="true" className="size-3.5" />
                </button>
              )}
              {!c.legacy && (
                <button
                  type="button"
                  onClick={() => onRemove(c.id)}
                  aria-label={`Delete ${c.label}`}
                  className="shrink-0 rounded p-0.5 text-ink-subtle hover:text-danger"
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

      <div className="flex flex-col gap-1.5 rounded-md border border-dashed border-line px-2 py-2">
        <div className="flex gap-1.5">
          <label htmlFor="new-credential-label" className="sr-only">
            Label for the new key
          </label>
          <input
            id="new-credential-label"
            value={label}
            onChange={(e) => setLabel(e.target.value)}
            placeholder="Label (e.g. work)"
            className="w-32 shrink-0 rounded border border-line bg-surface-raised px-2 py-1 text-[12px] text-ink placeholder:text-ink-subtle"
          />
          <label htmlFor="new-credential-secret" className="sr-only">
            New API key
          </label>
          <input
            id="new-credential-secret"
            type="password"
            value={secret}
            onChange={(e) => setSecret(e.target.value)}
            placeholder="Paste key"
            autoComplete="off"
            spellCheck={false}
            className="min-w-0 flex-1 rounded border border-line bg-surface-raised px-2 py-1 font-mono text-[12px] text-ink placeholder:text-ink-subtle"
          />
          <button
            type="button"
            onClick={submit}
            disabled={addDisabled || secret.trim() === ""}
            className="shrink-0 rounded border border-line px-2 py-1 text-[12px] font-medium text-ink hover:bg-surface-sunken disabled:opacity-40"
          >
            Add
          </button>
        </div>
        <p className="text-[11px] text-ink-subtle">
          Stored as <code>secrets/&lt;id&gt;.secret</code> (0600). Keys are never shown again after
          saving — only the last four characters.
        </p>
      </div>

      {error !== null && (
        <p role="alert" className="text-[11.5px] text-danger">
          {error}
        </p>
      )}
    </div>
  );
}
