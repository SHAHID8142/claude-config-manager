import { Check, Terminal, TriangleAlert } from "lucide-react";
import { LoadingButton } from "../components/LoadingButton";
import { Section } from "../components/Section";
import { StatusBanner } from "../components/StatusBanner";
import type { EnvironmentReport, RestoreOutcome } from "../lib/ipc";

export interface MaintenanceTabProps {
  environment: EnvironmentReport | null;
  restoring: boolean;
  onRestore: () => void;
  restoreOutcome: RestoreOutcome | null;
  restoreError: string | null;
  onDismissRestore: () => void;
  onDismissRestoreError: () => void;
  shellWarningDismissed: boolean;
  onDismissShellWarning: () => void;
}

/** "malformed json" → "Malformed JSON". */
function humanState(state: string): string {
  const words = state.replace(/_/g, " ");
  return words.charAt(0).toUpperCase() + words.slice(1);
}

/**
 * Maintenance screen: the checks the Dashboard summarises, the backups, and
 * every "restore" path. Nothing here writes provider settings.
 */
export function MaintenanceTab(props: MaintenanceTabProps) {
  const env = props.environment;

  return (
    <div className="flex flex-col gap-6">
      {props.restoreOutcome !== null && (
        <StatusBanner
          severity="success"
          message="Settings restored from the latest backup."
          detail={`Restored: ${props.restoreOutcome.restoredFrom} · pre-restore copy: ${props.restoreOutcome.preRestoreBackup || "(none — file was missing)"}`}
          onDismiss={props.onDismissRestore}
        />
      )}
      {props.restoreError !== null && (
        <StatusBanner
          severity="error"
          message="Restore failed."
          detail={props.restoreError}
          onDismiss={props.onDismissRestoreError}
        />
      )}

      <Section
        title="Environment"
        description="What this app found on the machine. Read-only: these checks never change a file."
      >
        {env === null ? (
          <p className="text-[12.5px] text-ink-muted">Checking the environment…</p>
        ) : (
          <dl className="grid gap-x-6 gap-y-3 sm:grid-cols-2">
            <div>
              <dt className="text-[11px] text-ink-subtle">Claude Code CLI</dt>
              <dd className="mt-0.5 flex items-center gap-1.5 text-[13px] text-ink">
                {env.cliFound ? (
                  <>
                    <Check aria-hidden="true" size={13} className="text-ok" />
                    {env.cliVersion ?? "found (version unknown)"}
                  </>
                ) : (
                  <>
                    <TriangleAlert aria-hidden="true" size={13} className="text-warn" />
                    not found on PATH
                  </>
                )}
              </dd>
            </div>
            <div>
              <dt className="text-[11px] text-ink-subtle">settings.json</dt>
              <dd className="mt-0.5 text-[13px] text-ink">{humanState(env.settingsState)}</dd>
            </div>
            <div>
              <dt className="text-[11px] text-ink-subtle">Config directory</dt>
              <dd className="mt-0.5 text-[13px] text-ink">
                {env.configDirWritable ? "writable" : "not writable"}
              </dd>
            </div>
            <div>
              <dt className="text-[11px] text-ink-subtle">Shell exports</dt>
              <dd className="mt-0.5 text-[13px] text-ink">
                {(env.shellEnv ?? []).length === 0
                  ? "none"
                  : (env.shellEnv ?? []).map((v) => v.name).join(", ")}
              </dd>
            </div>
            {env.settingsDetail !== null && (
              <div className="sm:col-span-2">
                <dt className="text-[11px] text-ink-subtle">Detail</dt>
                <dd className="mt-0.5 text-[12px] break-words text-ink-muted">{env.settingsDetail}</dd>
              </div>
            )}
            {env.cliVersionWarning !== null && (
              <div className="sm:col-span-2">
                <dt className="text-[11px] text-ink-subtle">Version note</dt>
                <dd className="mt-0.5 text-[12px] text-warn">{env.cliVersionWarning}</dd>
              </div>
            )}
          </dl>
        )}
      </Section>

      <Section
        title="Actions"
        description="Roll settings back to the newest backup. A pre-restore copy of the current file is taken first, so a restore is itself reversible."
      >
        <div className="flex flex-wrap items-center gap-3">
          <LoadingButton
            label="Restore last backup"
            pendingLabel="Restoring…"
            pending={props.restoring}
            variant="secondary"
            onClick={props.onRestore}
          />
          <p className="text-[12px] text-ink-muted">
            Backups live beside settings.json as settings.json.backup-*.
          </p>
        </div>
      </Section>

      <section className="flex items-start gap-3 rounded-[10px] border border-line bg-surface p-4">
        <span
          aria-hidden="true"
          className="mt-0.5 flex size-6 shrink-0 items-center justify-center rounded-[6px] bg-surface-sunken text-ink-muted"
        >
          <Terminal size={13} />
        </span>
        <div className="min-w-0 truncate text-[12px] leading-[1.6] text-ink-muted">
          <p className="font-medium text-ink">About the shell</p>
          <p>
            ANTHROPIC_* variables exported by your login shell override what settings.json says and
            cannot be changed from this app. The Dashboard warns when it sees them.
          </p>
        </div>
      </section>
    </div>
  );
}
