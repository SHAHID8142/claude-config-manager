import { useId } from "react";
import { FieldRow } from "./FieldRow";
import { Section } from "./Section";
import { focusRing } from "../lib/focus";

import type { BackupPreview } from "../lib/ipc";
export type { BackupPreview } from "../lib/ipc";

export interface RecoverySelectorProps {
  backups: BackupPreview[] | null;
  error?: string | null;
  selected: string | null;
  onSelect: (name: string) => void;
  loading?: boolean;
}

/** Display only the destination origin. A backup URL may contain a credential
 * in its userinfo, path, query or fragment; none belongs in the preview DOM. */
function endpointLabel(value: string | null): string {
  if (value === null) return "Native Anthropic (no endpoint override)";
  try {
    const endpoint = new URL(value);
    if (endpoint.protocol !== "https:" && endpoint.protocol !== "http:") {
      return "Endpoint unavailable";
    }
    return endpoint.origin;
  } catch {
    return "Endpoint unavailable";
  }
}

function backupDate(epoch: number): Date | null {
  const date = new Date(epoch * 1000);
  return epoch > 0 && Number.isFinite(date.getTime()) ? date : null;
}

function sizeLabel(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return "Size unavailable";
  return bytes < 1024 ? `${bytes} bytes` : `${Math.ceil(bytes / 1024).toLocaleString()} KiB`;
}

/** Controlled selection only. Choosing a snapshot does not restore it, change
 * its parent selection automatically, or expose the contents of settings JSON. */
export function RecoverySelector({
  backups,
  error = null,
  selected,
  onSelect,
  loading = false,
}: RecoverySelectorProps) {
  const id = useId();
  const busy = loading || backups === null;
  const entries = backups ?? [];
  const usable = entries.some((backup) => backup.valid);
  const chosen = entries.find((backup) => backup.fileName === selected);
  const preview = chosen?.valid ? chosen : null;
  const unavailableSelection = !busy && selected !== null && !preview;
  const selectionError = `${id}-selection-error`;
  const invalid = entries.filter((backup) => !backup.valid);
  const date = preview ? backupDate(preview.createdEpoch) : null;

  return (
    <Section
      title="Choose a backup"
      description="Preview a saved snapshot before restoring it."
      help={{
        title: "About backup selection",
        children:
          "Only backups containing a valid JSON object can be restored. This preview shows selected settings, not API keys. Helper presence does not verify the key or provider connection. Restore also replaces other settings in the snapshot; choosing here changes nothing until you confirm restore.",
      }}
    >
      <div className="min-w-0" aria-busy={busy}>
        {error && (
          <p role="alert" className="mb-2 break-words text-[12px] text-danger">
            {error}
          </p>
        )}
        <FieldRow
          label="Settings backup"
          htmlFor={id}
          hint="Choose a valid snapshot. Restoring requires a separate confirmation."
          help="Backups are listed in the order supplied by the app, normally newest first. Invalid snapshots are shown but cannot be selected. The current file is preserved before a restore."
        >
          <select
            id={id}
            name="settings-backup"
            value={preview?.fileName ?? ""}
            disabled={busy || !usable}
            aria-invalid={unavailableSelection || undefined}
            aria-describedby={unavailableSelection ? selectionError : undefined}
            className={`min-h-9 w-full min-w-0 rounded-md border border-control bg-surface px-2 text-[13px] text-ink disabled:cursor-not-allowed disabled:opacity-60 ${focusRing}`}
            onChange={(event) => {
              const candidate = entries.find((backup) => backup.fileName === event.target.value);
              if (candidate?.valid) onSelect(candidate.fileName);
            }}
          >
            <option value="" disabled>
              {busy ? "Reading backup previews…" : "Choose a valid backup"}
            </option>
            {entries.map((backup) => (
              <option key={backup.fileName} value={backup.fileName} disabled={!backup.valid}>
                {backup.fileName}
                {!backup.valid ? " — invalid" : ""}
              </option>
            ))}
          </select>
        </FieldRow>
        {unavailableSelection && (
          <p id={selectionError} className="mt-1 break-words text-[12px] text-danger">
            {chosen
              ? "The selected backup is invalid. Choose a valid snapshot to continue."
              : "The selected backup is no longer available. Choose another snapshot."}
          </p>
        )}
        {busy ? (
          <p className="mt-2 text-[12px] text-ink-muted">Reading backup previews…</p>
        ) : !usable ? (
          <p className="mt-2 text-[12px] text-ink-muted">
            No valid backups are available to restore.
          </p>
        ) : !preview ? (
          <p className="mt-2 text-[12px] text-ink-muted">
            Choose a backup to review its saved configuration.
          </p>
        ) : (
          <div className="mt-3 min-w-0 rounded-lg border border-line bg-surface p-3">
            <h3 className="text-[13px] font-medium text-ink">Selected backup</h3>
            <p className="mt-1 break-all font-mono text-[11px] text-ink-muted">
              {preview.fileName}
            </p>
            <dl className="mt-3 grid min-w-0 grid-cols-1 gap-3 text-[12px]">
              <div>
                <dt className="text-[11px] text-ink-muted">Saved at</dt>
                <dd className="mt-0.5 break-words text-ink">
                  {date ? (
                    <time dateTime={date.toISOString()}>{date.toLocaleString()}</time>
                  ) : (
                    "Time unavailable"
                  )}
                  {" · "}
                  {sizeLabel(preview.sizeBytes)}
                </dd>
              </div>
              <div className="min-w-0">
                <dt className="text-[11px] text-ink-muted">Model</dt>
                <dd className="mt-0.5 break-all font-mono text-ink">
                  {preview.model ?? "Provider default"}
                </dd>
              </div>
              <div className="min-w-0">
                <dt className="text-[11px] text-ink-muted">Destination</dt>
                <dd className="mt-0.5 break-all font-mono text-ink">
                  {endpointLabel(preview.baseUrl)}
                </dd>
              </div>
              <div>
                <dt className="text-[11px] text-ink-muted">Credential helper</dt>
                <dd
                  className={`mt-0.5 ${preview.helperPresent === false ? "text-warn" : "text-ink"}`}
                >
                  {preview.helperPresent === null
                    ? "Not configured in this snapshot"
                    : preview.helperPresent
                      ? "Found on this Mac"
                      : "Missing on this Mac — review before restoring"}
                </dd>
              </div>
            </dl>
            <p className="mt-3 text-[11px] leading-relaxed text-ink-muted">
              A valid snapshot does not prove its provider, model or API key will work. Destination
              paths and private URL values are hidden.
            </p>
          </div>
        )}
        {invalid.length > 0 && (
          <div className="mt-3 text-[12px]">
            <p className="font-medium text-warn">Snapshots that cannot be restored</p>
            <ul className="mt-1 max-h-32 space-y-2 overflow-y-auto rounded-md border border-line p-2">
              {invalid.map((backup) => (
                <li key={backup.fileName} className="break-words text-ink-muted">
                  <span className="break-all font-mono text-[11px]">{backup.fileName}</span>
                  {" — "}
                  <span>{backup.detail ?? "Not a readable settings JSON object."}</span>
                </li>
              ))}
            </ul>
          </div>
        )}
      </div>
    </Section>
  );
}
