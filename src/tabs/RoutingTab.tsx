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
  const proxyRunning = props.status?.running === true;
  // D6: one runtime state line for helper + proxy, one state-appropriate
  // action, Restart as a quiet secondary — no button soup.
  const runtimeUp = helper !== null && helper.running && proxyRunning;
  return (
    <div className="flex flex-col gap-6">
      {helper !== null && (
        <Section
          title="Runtime"
          description="A detached process that keeps the proxy up after this window closes."
        >
          <div className="flex flex-col gap-3">
            <div className="flex flex-wrap items-center gap-x-3 gap-y-1">
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
                {proxyRunning && props.status !== null && (
                  <span className="text-ink-muted">
                    {" · "}proxy on 127.0.0.1:{props.status.port ?? "…"}
                  </span>
                )}
              </p>
            </div>
            <div className="flex flex-wrap items-center gap-2">
              {runtimeUp || helper.running ? (
                <button
                  type="button"
                  onClick={props.onStopProxy}
                  disabled={props.busy || (!helper.running && !helper.stale)}
                  className="btn btn-secondary"
                >
                  Stop
                </button>
              ) : (
                <button
                  type="button"
                  onClick={props.onStartProxy}
                  disabled={props.busy || helper.running}
                  className="btn btn-primary"
                >
                  Start helper
                </button>
              )}
              <button
                type="button"
                onClick={props.onRestartProxy}
                disabled={props.busy}
                className="btn btn-secondary"
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
                  Installs a user LaunchAgent in ~/Library/LaunchAgents. No administrator rights,
                  and it can be removed here at any time.
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
        description="Tried top to bottom; failed targets cool down and are skipped."
        help={{
          title: "About the chain",
          children: (
            <p>
              Targets are tried in order. A target cools down after it fails, and cooling targets
              are skipped without ever leaving the chain empty.
            </p>
          ),
        }}
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

      {/* V3-28: the long explanation is one disclosure, not a paragraph on the
          working screen. The facts that change behaviour stay in the rows above. */}
      <details className="rounded-[10px] border border-line bg-surface px-4 py-3">
        <summary className="cursor-pointer text-[12.5px] font-medium text-ink marker:hidden">
          What the proxy does and does not do
        </summary>
        <ul className="mt-2 list-disc space-y-1 pl-4 text-[12px] leading-[1.6] text-ink-muted">
          <li>Listens on 127.0.0.1 only, with a bearer token Claude Code sends.</li>
          <li>Fails over on 429, 5xx, timeouts and dead connections, honouring Retry-After.</li>
          <li>Never retries a caller error (400/422), never replays a begun stream.</li>
          <li>Does not translate protocols: OpenAI-only providers still need a bridge.</li>
          <li>The helper keeps running after this window closes; see Proxy helper above.</li>
        </ul>
      </details>
    </div>
  );
}
