import { Route } from "lucide-react";
import { RoutingPanel } from "../components/RoutingPanel";
import { Section } from "../components/Section";
import type {
  CredentialView,
  HelperStatus,
  ProviderSummary,
  ProxyStatus,
  RoutingConfig,
  TargetHealth,
} from "../lib/ipc";

export interface RoutingTabProps {
  config: RoutingConfig | null;
  error: string | null;
  providers: ProviderSummary[];
  credentials: CredentialView[];
  onNeedCredentials: (providerId: string) => void;
  status: ProxyStatus | null;
  health: TargetHealth[] | null;
  checking: boolean;
  busy: boolean;
  onChange: (config: RoutingConfig) => void;
  onSave: () => void;
  onStartProxy: () => void;
  onStopProxy: () => void;
  /** V3-07/V3-09: detached helper state (proves routing works after the GUI quits). */
  helper: HelperStatus | null;
  onRestartProxy: () => void;
  onToggleLoginStart: (enabled: boolean) => void;
  onApplyRouted: () => void;
  onCheckHealth: () => void;
}

/**
 * Failover screen: the ordered chain, then the proxy that consumes it. The
 * panel is one component so the chain can never be saved half-edited.
 */
export function RoutingTab(props: RoutingTabProps) {
  const helper = props.helper;
  return (
    <div className="flex flex-col gap-6">
      {helper !== null && (
        <Section
          title="Proxy helper"
          description="Routed mode needs a process that stays up after this window closes. It listens on loopback only."
        >
          <div className="flex flex-col gap-3">
            <div className="flex items-center gap-2">
              <span
                aria-hidden="true"
                className={
                  helper.running
                    ? "size-2 rounded-full bg-ok"
                    : helper.stale
                      ? "size-2 rounded-full bg-warn"
                      : "size-2 rounded-full bg-ink-subtle"
                }
              />
              <p className="text-[13px] text-ink" role="status">
                {helper.message}
              </p>
            </div>
            <div className="flex flex-wrap items-center gap-2">
              <button
                type="button"
                onClick={props.onStartProxy}
                disabled={props.busy || helper.running}
                className="h-8 rounded-[6px] border border-control bg-surface px-3 text-[12.5px] text-ink hover:bg-surface-hover disabled:opacity-50"
              >
                Start helper
              </button>
              <button
                type="button"
                onClick={props.onStopProxy}
                disabled={props.busy || (!helper.running && !helper.stale)}
                className="h-8 rounded-[6px] border border-control bg-surface px-3 text-[12.5px] text-ink hover:bg-surface-hover disabled:opacity-50"
              >
                Stop helper
              </button>
              <button
                type="button"
                onClick={props.onRestartProxy}
                disabled={props.busy}
                className="h-8 rounded-[6px] border border-control bg-surface px-3 text-[12.5px] text-ink hover:bg-surface-hover disabled:opacity-50"
              >
                Restart
              </button>
            </div>
            <label className="flex items-start gap-2 text-[12.5px] text-ink">
              <input
                type="checkbox"
                checked={helper.loginStart}
                onChange={(e) => props.onToggleLoginStart(e.target.checked)}
                disabled={props.busy}
                className="mt-0.5"
              />
              <span>
                Start the helper when I log in
                <span className="block text-[11.5px] text-ink-muted">
                  Installs a user LaunchAgent in ~/Library/LaunchAgents. No administrator
                  rights, and it can be removed here at any time.
                </span>
              </span>
            </label>
            {helper.stale && (
              <p className="text-[12px] text-warn">
                The helper is not answering. Restart it before relying on routed mode.
              </p>
            )}
          </div>
        </Section>
      )}
      <Section
        title="Chain"
        description="Try these targets in order. A target cools down after it fails, and cooling targets are skipped without ever leaving the chain empty."
      >
        {props.config === null ? (
          <p className="text-[12.5px] text-ink-muted">Reading routing configuration…</p>
        ) : (
          <RoutingPanel
            config={props.config}
            providers={props.providers}
            credentials={props.credentials}
            onNeedCredentials={props.onNeedCredentials}
            status={props.status}
            health={props.health}
            checking={props.checking}
            busy={props.busy}
            error={props.error}
            onChange={props.onChange}
            onSave={props.onSave}
            onStartProxy={props.onStartProxy}
            onStopProxy={props.onStopProxy}
            onApplyRouted={props.onApplyRouted}
            onCheckHealth={props.onCheckHealth}
          />
        )}
      </Section>

      <section className="flex items-start gap-3 rounded-[10px] border border-line bg-surface p-4">
        <span
          aria-hidden="true"
          className="mt-0.5 flex size-6 shrink-0 items-center justify-center rounded-[6px] bg-surface-sunken text-ink-muted"
        >
          <Route size={13} />
        </span>
        <div className="min-w-0 truncate text-[12px] leading-[1.6] text-ink-muted">
          <p className="font-medium text-ink">What the proxy does and does not do</p>
          <ul className="mt-1 list-disc space-y-1 pl-4">
            <li>Listens on 127.0.0.1 only, with a per-launch token Claude Code sends.</li>
            <li>Fails over on 429, 5xx, timeouts and dead connections — honouring Retry-After.</li>
            <li>Never retries a caller error (400/422), never replays a begun stream, protects charges.</li>
            <li>Does not translate protocols: OpenAI-only providers still need a bridge.</li>
            <li>Runs while this app is open; Claude Code needs it for routed mode.</li>
          </ul>
        </div>
      </section>
    </div>
  );
}
