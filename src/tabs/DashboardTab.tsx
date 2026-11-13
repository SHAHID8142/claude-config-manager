import { useEffect, useRef, useState, type ReactNode } from "react";
import { AlertTriangle, KeyRound, Layers, Route, Sparkles, Terminal } from "lucide-react";
import { CapabilityBadge, BridgeBadge } from "../components/CapabilityBadge";
import { StatusBanner } from "../components/StatusBanner";
import { contextWindowLabel, modeLabel, needsAttention, warningText } from "../lib/statusText";
import type { TabId } from "../lib/tab";
import type {
  ApplyOutcome,
  CredentialView,
  EffectiveStatus,
  EnvironmentReport,
  ProviderSummary,
  ProxyStatus,
  RoutingConfig,
} from "../lib/ipc";

export interface DashboardTabProps {
  status: EffectiveStatus | null;
  providers: ProviderSummary[] | null;
  selected: ProviderSummary | null;
  credentials: CredentialView[];
  routing: RoutingConfig | null;
  proxy: ProxyStatus | null;
  environment: EnvironmentReport | null;
  applyOutcome: ApplyOutcome | null;
  applyError: string | null;
  onDismissApply: () => void;
  onDismissApplyError: () => void;
  onNavigate: (tab: TabId) => void;
}

type Tone = "neutral" | "ok" | "warn" | "accent";

const TONE_CHIP: Record<Tone, string> = {
  neutral: "bg-surface-sunken text-ink-muted",
  ok: "bg-ok-soft text-ok",
  warn: "bg-warn-soft text-warn",
  accent: "bg-accent-soft text-accent",
};

/**
 * A dashboard card: icon chip, title, one big value, one line of context, and
 * an optional action that opens the screen where it is changed. Flat surface,
 * separated by a 1px line — no shadow, no gradient.
 */
function Card({
  icon,
  title,
  tone = "neutral",
  action,
  children,
}: {
  icon: ReactNode;
  title: string;
  tone?: Tone;
  action?: { label: string; tab: TabId; onNavigate: (tab: TabId) => void };
  children: ReactNode;
}) {
  return (
    <section
      aria-label={title}
      className="flex flex-col rounded-[10px] border border-line bg-surface p-4"
    >
      <div className="flex items-center gap-2">
        <span
          aria-hidden="true"
          className={`flex size-6 shrink-0 items-center justify-center rounded-[6px] ${TONE_CHIP[tone]}`}
        >
          {icon}
        </span>
        <h2 className="text-[12px] font-medium text-ink-muted">{title}</h2>
        {action !== undefined && (
          <button
            type="button"
            onClick={() => action.onNavigate(action.tab)}
            className="ml-auto text-[12px] text-ink-muted underline-offset-2 hover:text-ink hover:underline"
          >
            {action.label}
          </button>
        )}
      </div>
      <div className="mt-3 flex flex-1 flex-col gap-1">{children}</div>
    </section>
  );
}

/** Big value inside a card: the one thing the card answers. */
function Value({ children, mono }: { children: ReactNode; mono?: boolean }) {
  return (
    <p
      className={`text-[15px] leading-[1.35] font-medium break-all text-ink ${mono ? "font-mono text-[13px]" : ""}`}
    >
      {children}
    </p>
  );
}

/** Quiet supporting line. */
function Note({ children }: { children: ReactNode }) {
  return <p className="text-[12px] leading-[1.5] text-ink-muted">{children}</p>;
}

/**
 * The screen's answer: what Claude Code is using, what is stored, and what is
 * wrong — each card carries the way to fix it.
 */
export function DashboardTab(props: DashboardTabProps) {
  // Signature move (DESIGN.md): the hero card sweeps once when an apply
  // succeeds — "the file changed underneath you, and it's safe".
  const [sweeping, setSweeping] = useState(false);
  const lastOutcome = useRef(props.applyOutcome);
  useEffect(() => {
    if (props.applyOutcome !== null && props.applyOutcome !== lastOutcome.current) {
      setSweeping(true);
      const t = window.setTimeout(() => setSweeping(false), 750);
      return () => window.clearTimeout(t);
    }
    lastOutcome.current = props.applyOutcome;
  }, [props.applyOutcome]);

  const { status, selected, credentials, routing, proxy, environment } = props;

  const configured = status !== null && status.settingsFound;
  const warned = needsAttention(status);
  const activeCredential =
    credentials.find((c) => c.enabled) ?? (credentials.length > 0 ? credentials[0] : null);
  const targets = routing?.targets ?? [];
  const enabledTargets = targets.filter((t) => t.enabled);
  const proxyRunning = proxy !== null && proxy.running;
  const proxyUrl = status?.envBaseUrl?.startsWith("http://127.0.0.1") ?? false;

  // D2: with no in-app selection, a helper-configured provider is still
  // knowable — the helper file name is the backend's own provider record.
  const configuredProvider =
    selected === null && status?.apiKeyHelper != null
      ? (props.providers?.find(
          (p) =>
            p.id ===
            status.apiKeyHelper!.replace(/\\/g, "/").split("/").pop()?.replace(/\.sh$/, ""),
        ) ?? null)
      : null;

  return (
    <div className="flex flex-col gap-4">
      {props.applyOutcome !== null && (
        <StatusBanner
          severity="success"
          message="Configuration applied and verified."
          detail={`Backup: ${props.applyOutcome.backupFileName || "(none — first run)"} · helper: ${props.applyOutcome.helperPath}`}
          onDismiss={props.onDismissApply}
        />
      )}
      {props.applyError !== null && (
        <StatusBanner
          severity="error"
          message="Save & Apply failed."
          detail={props.applyError}
          onDismiss={props.onDismissApplyError}
        />
      )}

      {environment !== null && !environment.cliFound && (
        <StatusBanner
          severity="warning"
          message="Claude Code CLI not found on PATH."
          detail="Settings can still be written; Claude Code will read them on its next launch."
        />
      )}

      <div className="grid gap-4 md:grid-cols-2">
        {/* The whole point of the app, so it spans the width and speaks first. */}
        <section
          aria-label="Active configuration"
          className="relative overflow-hidden rounded-[10px] border border-line bg-surface p-4 md:col-span-2"
        >
          {sweeping && (
            <span
              aria-hidden="true"
              className="motion-safe:animate-sweep absolute inset-x-0 top-0 h-[2px] bg-accent"
            />
          )}
          <div className="flex items-start justify-between gap-3">
            <div>
              <p className="text-[12px] text-ink-muted">Claude Code is using</p>
              <p className="mt-1.5 flex items-center gap-2 text-[19px] font-semibold tracking-[-0.01em] text-ink">
                <span
                  aria-hidden="true"
                  className={`size-2 shrink-0 rounded-full ${warned ? "bg-warn" : configured ? "bg-ok" : "bg-ink-subtle"}`}
                />
                {status === null
                  ? "Reading the current configuration…"
                  : configured
                    ? modeLabel(status)
                    : "not configured yet"}
              </p>
            </div>
            <span className="shrink-0 text-[11px] text-ink-subtle">live from settings.json</span>
          </div>

          {configured ? (
            <dl className="mt-4 grid grid-cols-2 gap-x-6 gap-y-3 border-t border-line pt-4 sm:grid-cols-4">
              <div className="min-w-0 truncate">
                <dt className="text-[11px] text-ink-subtle">Model</dt>
                <dd
                  className="mt-0.5 truncate font-mono text-[12.5px] text-ink"
                  title={status.envModel ?? status.model ?? "not set"}
                >
                  {status.envModel ?? status.model ?? "not set"}
                </dd>
              </div>
              <div className="min-w-0 truncate">
                <dt className="text-[11px] text-ink-subtle">Endpoint</dt>
                <dd
                  className="mt-0.5 truncate text-[13px] text-ink"
                  title={status.envBaseUrl ?? undefined}
                >
                  {status.envBaseUrl ?? "native Anthropic"}
                </dd>
              </div>
              <div className="min-w-0 truncate">
                <dt className="text-[11px] text-ink-subtle">Key</dt>
                <dd className="mt-0.5 truncate text-[13px] text-ink">
                  {status.apiKeyHelper !== null
                    ? "credential file"
                    : status.apiKeyPresent || status.authTokenPresent
                      ? "plaintext env"
                      : "none"}
                </dd>
              </div>
              <div className="min-w-0 truncate">
                <dt className="text-[11px] text-ink-subtle">Context window</dt>
                <dd className="mt-0.5 truncate text-[13px] tabular-nums text-ink">
                  {contextWindowLabel(status.envMaxContextTokens)}
                </dd>
              </div>
            </dl>
          ) : (
            <p className="mt-4 border-t border-line pt-4 text-[13px] text-ink-muted">
              {status === null
                ? "Reading settings.json…"
                : "No settings file yet — the first Save & Apply creates it."}
            </p>
          )}

          {configured && warned && (
            <ul className="mt-3 flex flex-col gap-1.5 border-t border-line pt-3">
              {status.warnings.map((w, i) => (
                <li key={i} className="flex items-start gap-2 text-[12px] text-warn">
                  <AlertTriangle aria-hidden="true" size={13} className="mt-0.5 shrink-0" />
                  {warningText(w)}
                </li>
              ))}
            </ul>
          )}
        </section>

        <Card
          icon={<Sparkles size={13} aria-hidden="true" />}
          title="Provider"
          tone={selected === null ? "neutral" : "accent"}
          action={{ label: "Change", tab: "provider", onNavigate: props.onNavigate }}
        >
          <Value>
            {selected !== null
              ? selected.name
              : configuredProvider !== null
                ? configuredProvider.name
                : status?.apiKeyHelper !== null && status?.apiKeyHelper !== undefined
                  ? "configured via helper"
                  : "No provider selected"}
          </Value>
          {selected === null && configuredProvider !== null ? (
            <Note>Managed elsewhere in this app session — open Provider to take over.</Note>
          ) : selected !== null ? (
            <>
              <Note>
                {selected.protocol === "anthropic_native"
                  ? "Anthropic-native endpoint"
                  : selected.protocol === "anthropic_compat"
                    ? "Anthropic-compatible endpoint"
                    : "OpenAI-only — needs a bridge"}
              </Note>
              <div className="mt-1 flex flex-wrap items-center gap-1.5">
                <CapabilityBadge
                  status={selected.capabilityStatus.status}
                  sourceUrl={
                    selected.capabilityStatus.status === "verified"
                      ? selected.capabilityStatus.source_url
                      : null
                  }
                />
                <BridgeBadge required={selected.bridge === "required_for_open_ai_only"} />
              </div>
            </>
          ) : (
            <Note>Pick a provider to write settings.</Note>
          )}
        </Card>

        <Card
          icon={<Layers size={13} aria-hidden="true" />}
          title="Model"
          tone="neutral"
          action={{ label: "Choose", tab: "model", onNavigate: props.onNavigate }}
        >
          <Value mono>{status?.envModel ?? status?.model ?? "not set"}</Value>
          <Note>
            Window: {contextWindowLabel(status?.envMaxContextTokens ?? null)}
            {status?.envDefaultHaiku || status?.envDefaultSonnet || status?.envDefaultOpus
              ? " · role overrides set"
              : ""}
          </Note>
        </Card>

        <Card
          icon={<KeyRound size={13} aria-hidden="true" />}
          title="Credentials"
          tone={credentials.length > 0 ? "ok" : "warn"}
          action={{ label: "Manage", tab: "credentials", onNavigate: props.onNavigate }}
        >
          <Value>
            {credentials.length === 0
              ? "No stored key"
              : `${credentials.length} stored ${credentials.length === 1 ? "key" : "keys"}`}
          </Value>
          <Note>
            {activeCredential === null
              ? "Save & Apply needs a key."
              : `Using ${activeCredential.label}${activeCredential.last4 !== "" ? ` · …${activeCredential.last4}` : ""}`}
          </Note>
        </Card>

        <Card
          icon={<Route size={13} aria-hidden="true" />}
          title="Failover"
          tone={proxyRunning ? "ok" : "neutral"}
          action={{ label: "Open", tab: "routing", onNavigate: props.onNavigate }}
        >
          <Value>
            {routing === null
              ? "Reading…"
              : routing.enabled
                ? `${enabledTargets.length} ${enabledTargets.length === 1 ? "target" : "targets"} in order`
                : "Off"}
          </Value>
          <Note>
            {proxyRunning
              ? `Proxy running on port ${proxy?.port ?? "?"}`
              : proxyUrl
                ? "Claude Code points at the proxy, which is not running."
                : "Claude Code talks to the provider directly."}
          </Note>
        </Card>

        <Card
          icon={<Terminal size={13} aria-hidden="true" />}
          title="Environment"
          tone={environment === null ? "neutral" : environment.cliFound ? "ok" : "warn"}
          action={{ label: "Details", tab: "maintenance", onNavigate: props.onNavigate }}
        >
          <Value>
            {environment === null
              ? "Checking…"
              : environment.cliFound
                ? `Claude Code ${environment.cliVersion ?? "(version unknown)"}`
                : "CLI not found"}
          </Value>
          <Note>
            {environment === null
              ? "Reading the shell and settings file."
              : `settings.json: ${environment.settingsState.replace(/_/g, " ")}${
                  (environment.shellEnv ?? []).length > 0
                    ? ` · ${(environment.shellEnv ?? []).length} ANTHROPIC_* exported by the shell`
                    : ""
                }`}
          </Note>
        </Card>
      </div>
    </div>
  );
}
