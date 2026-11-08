import { Route } from "lucide-react";
import { RoutingPanel } from "../components/RoutingPanel";
import { Section } from "../components/Section";
import type {
  CredentialView,
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
  onApplyRouted: () => void;
  onCheckHealth: () => void;
}

/**
 * Failover screen: the ordered chain, then the proxy that consumes it. The
 * panel is one component so the chain can never be saved half-edited.
 */
export function RoutingTab(props: RoutingTabProps) {
  return (
    <div className="flex flex-col gap-6">
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
