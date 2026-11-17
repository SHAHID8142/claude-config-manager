import { SwitchHistory } from "../components/SwitchHistory";
import { ProviderGroups } from "../components/ProviderGroups";
import { HelpPopover } from "../components/HelpPopover";
import { defaultIpc, type IpcLike } from "../lib/ipc";
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
  ipc?: IpcLike;
  onAddProvider?: () => void;
  config: RoutingConfig | null;
  dirty?: boolean;
  receipt?: string | null;
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
                Restart &amp; clear pauses
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
            <div className="text-[12px] text-ink-muted">
              About login start{" "}
              <HelpPopover title="Start at login">
                The optional user LaunchAgent starts the local helper after login. Turning it off
                removes that registration; it does not delete stored keys.
              </HelpPopover>
            </div>
            {helper.stale && (
              <p className="text-[12px] text-warn">
                The helper is not answering. Restart it before relying on routed mode.
              </p>
            )}
          </div>
        </Section>
      )}
      <Section
        title="Switching order"
        description="Keys within one provider first. Then the next provider."
      >
        <div className="mb-4 rounded-lg bg-surface-sunken p-3 text-[12px] leading-relaxed text-ink-muted">
          Provider A: Key 1 → Key 2 → Key 3 <span className="mx-1">→</span> Provider B: Key 1 → Key
          2.
          <br />A working key keeps serving requests. A failure moves to the next usable key.
        </div>
        {props.error && (
          <p role="alert" className="mb-3 text-[12px] text-danger">
            {props.error}
          </p>
        )}
        {props.config ? (
          <ProviderGroups
            config={props.config}
            providers={props.providers}
            ipc={props.ipc ?? defaultIpc}
            busy={props.busy}
            onChange={props.onChange}
          />
        ) : (
          <p className="text-[12px]">
            {props.error
              ? "Switching settings are unavailable. Refresh to retry."
              : "Reading switching settings…"}
          </p>
        )}
        <p role="status" className="mt-3 text-[12px] text-ink-muted">
          {props.dirty
            ? "Pending provider/model changes — save or enable to apply this policy."
            : (props.receipt ?? "Showing saved provider policy.")}
        </p>
        <p className="mt-3 text-[12px] text-ink-muted">
          Review the provider order, model and endpoint above. Enable saves this policy and points
          global Claude Code settings at the local helper. Existing unrelated settings are
          preserved.
        </p>
        <div className="mt-4 flex flex-wrap gap-2">
          <button
            type="button"
            className="btn btn-secondary"
            disabled={props.busy}
            onClick={props.onAddProvider}
          >
            Add a provider
          </button>
          <button
            type="button"
            className="btn btn-secondary"
            disabled={props.busy || !props.config?.targets.length}
            onClick={props.onSave}
          >
            Save order
          </button>
          <button
            type="button"
            className="btn btn-primary"
            disabled={
              props.busy || !props.config?.targets.some((t) => t.enabled && t.model?.trim())
            }
            onClick={props.onApplyRouted}
          >
            {props.busy ? "Saving…" : "Save & enable automatic switching"}
          </button>
        </div>
        <p className="mt-2 text-[11px] text-ink-muted">
          Provider/model edits are pending until saved. Stored-key changes apply to the next routed
          request.
        </p>
      </Section>
      <Section
        title="Recent activity"
        description="Observed helper activity, not a billing statement."
      >
        <p className="text-[12px] text-ink-muted">
          {props.status
            ? `${props.status.requests} requests · ${props.status.fallbacks} fallback attempts · ${props.status.failures} failures`
            : "No helper activity available."}
        </p>
        {props.status?.lastTarget && (
          <p className="mt-1 break-all text-[12px]">
            Last response key: {props.status.lastKeyLabel ?? props.status.lastTarget}
          </p>
        )}
        {props.status?.lastSwitchReason && (
          <p className="mt-1 text-[12px] text-ink-muted">
            Last switch: {props.status.lastSwitchReason}
          </p>
        )}
        {props.status?.cooldownSeconds && Object.keys(props.status.cooldownSeconds).length > 0 && (
          <p className="mt-1 text-[12px] text-warn">
            Next paused key eligible in {Math.min(...Object.values(props.status.cooldownSeconds))}{" "}
            seconds. Restart the helper to explicitly clear pauses.
          </p>
        )}
        <SwitchHistory ipc={props.ipc ?? defaultIpc} />
        {props.status?.activityError && (
          <p role="alert" className="text-[12px] text-danger">
            {props.status.activityError}
          </p>
        )}
        {props.status?.lastError && (
          <p className="mt-1 text-[12px] text-danger">{props.status.lastError}</p>
        )}
        {props.status?.cooling.length ? (
          <p className="mt-1 break-all text-[12px] text-warn">
            Paused after failure: {props.status.cooling.join(", ")}
          </p>
        ) : null}
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
          <li>
            NVIDIA chat models use built-in translation. Other OpenAI-only providers need an
            available translator.
          </li>
          <li>The helper keeps running after this window closes; see Proxy helper above.</li>
        </ul>
      </details>
    </div>
  );
}
