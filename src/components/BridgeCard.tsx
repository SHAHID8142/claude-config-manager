/** Bridge setup card (T48/T52/T54): consent → install → run → route.
 * One state machine per surface, every state honest: the card never
 * pretends the bridge is installed or running. Consent is recorded
 * server-side; the UI only mirrors `overview.consented`. */
import { useCallback, useEffect, useState } from "react";
import { Section } from "./Section";
import { LoadingButton } from "./LoadingButton";
import { StatusBanner } from "./StatusBanner";
import { focusRing } from "../lib/focus";
import {
  bridgeApplyMode,
  bridgeConsentInfo,
  bridgeConsentRecord,
  bridgeConsentWithdraw,
  bridgeInstall,
  bridgeOverview,
  bridgeStart,
  bridgeStop,
  bridgeUninstall,
  isCommandError,
  type ApplyOutcome,
  type BridgeOverview,
  type BridgePin,
  type IpcLike,
} from "../lib/ipc";

export interface BridgeCardProps {
  ipc: IpcLike;
  /** T54 context: the OpenAI-only provider the bridge would serve. */
  providerId: string | null;
  providerName: string | null;
  model: string | null;
  onApplied: (outcome: ApplyOutcome) => void;
}

type Phase =
  | { kind: "loading" }
  | { kind: "consent"; pin: BridgePin; overview: BridgeOverview }
  | { kind: "ready"; pin: BridgePin; overview: BridgeOverview };

function mb(bytes: number): string {
  return `${(bytes / 1024 / 1024).toFixed(1)} MiB`;
}

export function BridgeCard({ ipc, providerId, providerName, model, onApplied }: BridgeCardProps) {
  const [phase, setPhase] = useState<Phase>({ kind: "loading" });
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [applyOutcome, setApplyOutcome] = useState<ApplyOutcome | null>(null);

  const refresh = useCallback(async () => {
    const [pin, overview] = await Promise.all([bridgeConsentInfo(ipc), bridgeOverview(ipc)]);
    setPhase(
      overview.consented ? { kind: "ready", pin, overview } : { kind: "consent", pin, overview },
    );
  }, [ipc]);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [pin, overview] = await Promise.all([
          bridgeConsentInfo(ipc),
          bridgeOverview(ipc),
        ]);
        if (!cancelled) {
          setPhase(
            overview.consented
              ? { kind: "ready", pin, overview }
              : { kind: "consent", pin, overview },
          );
        }
      } catch (e: unknown) {
        if (!cancelled) {
          setError(isCommandError(e) ? e.message : "Bridge status unavailable.");
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [ipc]);

  async function run(name: string, fn: () => Promise<void>) {
    setBusy(name);
    setError(null);
    try {
      await fn();
      await refresh();
    } catch (e: unknown) {
      setError(isCommandError(e) ? e.message : "Bridge operation failed.");
    } finally {
      setBusy(null);
    }
  }

  const routing = phase.kind === "ready" && phase.overview.running && providerId !== null;

  return (
    <Section
      title="Bridge"
      description="Translates OpenAI-only providers for Claude Code."
      help={{
        title: "About the bridge",
        children: (
          <p>
            The bridge is a pinned third-party package (MIT) installed locally under your
            application-support directory. It listens on 127.0.0.1 only and translates
            Anthropic-protocol requests for OpenAI-only providers. Nothing is installed or executed
            before you consent, and you can remove it here at any time.
          </p>
        ),
      }}
    >
      <div className="flex flex-col gap-3">
        {error !== null && (
          <StatusBanner
            severity="error"
            message="Bridge error."
            detail={error}
            onDismiss={() => setError(null)}
          />
        )}
        {applyOutcome !== null && (
          <StatusBanner
            severity="success"
            message="Claude Code is routed through the bridge."
            detail={`Backup: ${applyOutcome.backupFileName}`}
            onDismiss={() => setApplyOutcome(null)}
          />
        )}

        {phase.kind === "loading" && (
          <p className="text-[12.5px] text-ink-muted">Checking bridge…</p>
        )}

        {phase.kind === "consent" && (
          <div className="flex flex-col gap-3 rounded-[8px] border border-line bg-surface-muted p-3.5">
            <p className="text-[13px] font-medium text-ink">Install the bridge?</p>
            <dl className="grid gap-x-6 gap-y-1 text-[12px] sm:grid-cols-[120px_minmax(0,1fr)]">
              <dt className="text-ink-subtle">Package</dt>
              <dd className="font-mono text-ink">{phase.pin.package}</dd>
              <dt className="text-ink-subtle">Version</dt>
              <dd className="text-ink tabular-nums">{phase.pin.version} (pinned)</dd>
              <dt className="text-ink-subtle">License</dt>
              <dd className="text-ink">{phase.pin.license}</dd>
              <dt className="text-ink-subtle">Download</dt>
              <dd className="text-ink">{`${mb(phase.pin.tarballBytes)}, sha512-verified before install`}</dd>
              <dt className="text-ink-subtle">Dependencies</dt>
              <dd className="text-ink">installed from a pinned lockfile (npm ci)</dd>
              <dt className="text-ink-subtle">Listens on</dt>
              <dd className="text-ink">127.0.0.1 only</dd>
            </dl>
            <p className="text-[11.5px] leading-[1.6] text-ink-muted">
              Installing runs a third-party program (never as administrator, never outside your user
              directory). You can remove it here at any time.
            </p>
            <div className="flex items-center gap-2">
              <button
                type="button"
                disabled={busy !== null}
                onClick={() =>
                  run("consent", async () => {
                    await bridgeConsentRecord(ipc);
                  })
                }
                className={`btn btn-primary ${focusRing}`}
              >
                I understand — enable the bridge
              </button>
            </div>
          </div>
        )}

        {phase.kind === "ready" && (
          <div className="flex flex-col gap-3">
            <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-[13px]">
              <span
                aria-hidden="true"
                className={`size-2 rounded-full ${phase.overview.running ? "bg-ok" : phase.overview.installed ? "bg-accent" : "bg-ink-subtle"}`}
              />
              <span className="text-ink" role="status">
                {!phase.overview.installed
                  ? "Installed? not yet"
                  : phase.overview.running
                    ? `running on 127.0.0.1:${phase.overview.gatewayPort}`
                    : "installed, stopped"}
              </span>
              <span className="text-[11.5px] text-ink-subtle tabular-nums">
                {phase.overview.nodeFound
                  ? `Node ${phase.overview.nodeMajor} found`
                  : `Node ${phase.pin.minNodeMajor}+ not found`}
              </span>
            </div>

            <div className="flex flex-wrap gap-2">
              {!phase.overview.installed && (
                <LoadingButton
                  label="Install"
                  pendingLabel="Installing…"
                  pending={busy === "install"}
                  disabled={!phase.overview.nodeFound}
                  variant="secondary"
                  onClick={() =>
                    run("install", async () => {
                      await bridgeInstall(ipc);
                    })
                  }
                />
              )}
              {phase.overview.installed && !phase.overview.running && (
                <LoadingButton
                  label="Start"
                  pendingLabel="Starting…"
                  pending={busy === "start"}
                  variant="secondary"
                  onClick={() =>
                    run("start", async () => {
                      await bridgeStart(ipc);
                    })
                  }
                />
              )}
              {phase.overview.running && (
                <LoadingButton
                  label="Stop"
                  pendingLabel="Stopping…"
                  pending={busy === "stop"}
                  variant="secondary"
                  onClick={() =>
                    run("stop", async () => {
                      await bridgeStop(ipc);
                    })
                  }
                />
              )}
              {phase.overview.installed && (
                <LoadingButton
                  label="Uninstall"
                  pendingLabel="Removing…"
                  pending={busy === "uninstall"}
                  variant="secondary"
                  onClick={() =>
                    run("uninstall", async () => {
                      await bridgeUninstall(ipc);
                      await bridgeConsentWithdraw(ipc);
                    })
                  }
                />
              )}
              {routing && providerName !== null && (
                <LoadingButton
                  label={`Route ${providerName} through bridge`}
                  pendingLabel="Applying…"
                  pending={busy === "route"}
                  variant="primary"
                  onClick={() =>
                    run("route", async () => {
                      const outcome = await bridgeApplyMode(
                        providerId ?? "",
                        model ?? undefined,
                        ipc,
                      );
                      setApplyOutcome(outcome);
                      onApplied(outcome);
                    })
                  }
                />
              )}
            </div>
            {routing && (
              <p className="text-[11.5px] text-ink-muted">
                Claude Code will use 127.0.0.1:{phase.overview.gatewayPort} — the key stays in your
                owner-only credential file.
              </p>
            )}
          </div>
        )}
      </div>
    </Section>
  );
}
