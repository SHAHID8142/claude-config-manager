import { KeyRound } from "lucide-react";
import { CredentialsPanel } from "../components/CredentialsPanel";
import { Section } from "../components/Section";
import type { CredentialView, ProviderSummary } from "../lib/ipc";

export interface CredentialsTabProps {
  provider: ProviderSummary | null;
  isCustom: boolean;
  credentials: CredentialView[];
  error: string | null;
  addDisabled: boolean;
  onAdd: (label: string, secret: string) => void;
  onRemove: (id: string) => void;
  onToggle: (id: string, enabled: boolean) => void;
  onPromote: (id: string) => void;
  /** V3-26: the directory the backend actually writes, never a guess. */
  secretsDir: string | null;
  /** D5: guided route back to the Provider screen when nothing is selected. */
  onNavigateToProvider: () => void;
}

/**
 * Credentials screen: several keys for the same provider, ordered. Claude Code
 * is handed the first *enabled* one; the rest are spares you can promote.
 */
export function CredentialsTab(props: CredentialsTabProps) {
  const enabled = props.credentials.filter((c) => c.enabled);
  const active = enabled[0] ?? null;
  const name = props.isCustom ? "this custom provider" : (props.provider?.name ?? "no provider");
  const secretsDir = props.secretsDir ?? "~/.claude-config-manager/secrets";

  // D5: this screen is provider-scoped. Without a provider the honest state
  // is a guided hand-off to Provider — never zeroed stats or "no provider".
  if (props.provider === null && !props.isCustom) {
    return (
      <section
        aria-label="Credentials"
        className="mx-auto mt-16 flex max-w-md flex-col items-center gap-3 rounded-[12px] border border-line bg-surface p-8 text-center"
      >
        <span
          aria-hidden="true"
          className="flex size-10 items-center justify-center rounded-[10px] bg-accent-soft text-accent"
        >
          <KeyRound size={18} />
        </span>
        <h2 className="text-[15px] font-semibold text-ink">Choose a provider first</h2>
        <p className="text-[13px] leading-[1.6] text-ink-muted">
          Keys belong to a provider. Pick one on the Provider screen — custom providers work too —
          and your keys will show up here.
        </p>
        <button
          type="button"
          onClick={props.onNavigateToProvider}
          className="btn btn-primary mt-1"
        >
          Go to Provider
        </button>
      </section>
    );
  }

  return (
    <div className="flex flex-col gap-6">
      <dl className="grid grid-cols-3 gap-4 rounded-[10px] border border-line bg-surface p-4">
        <div>
          <dt className="text-[11px] text-ink-subtle">Stored</dt>
          <dd className="mt-0.5 flex items-baseline gap-1.5">
            <span className="text-[20px] leading-none font-semibold tabular-nums text-ink">
              {props.credentials.length}
            </span>
            <span className="text-[12px] text-ink-muted">
              {props.credentials.length === 1 ? "key" : "keys"}
            </span>
          </dd>
        </div>
        <div>
          <dt className="text-[11px] text-ink-subtle">Enabled</dt>
          <dd className="mt-0.5 flex items-baseline gap-1.5">
            <span className="text-[20px] leading-none font-semibold tabular-nums text-ink">
              {enabled.length}
            </span>
            <span className="text-[12px] text-ink-muted">
              {enabled.length === 1 ? "key in use" : "keys eligible"}
            </span>
          </dd>
        </div>
        <div className="min-w-0 truncate">
          <dt className="text-[11px] text-ink-subtle">Handed to Claude Code</dt>
          <dd className="mt-1 text-[13px] text-ink" title={active?.label}>
            {active === null ? "nothing yet" : active.label}
          </dd>
        </div>
      </dl>

      <Section
        title={`Keys for ${name}`}
        description="The first enabled key is used; the rest are spares."
        help={{
          title: "About multiple keys",
          children: (
            <p>
              Add as many keys as you like. Promoting moves a key to the front. A legacy key stays
              where it is until you replace it.
            </p>
          ),
        }}
      >
        <CredentialsPanel
          credentials={props.credentials}
          addDisabled={props.addDisabled}
          onAdd={props.onAdd}
          onRemove={props.onRemove}
          onToggle={props.onToggle}
          onPromote={props.onPromote}
          error={props.error}
        />
      </Section>

      <section className="flex items-start gap-3 rounded-[10px] border border-line bg-surface p-4">
        <span
          aria-hidden="true"
          className="mt-0.5 flex size-6 shrink-0 items-center justify-center rounded-[6px] bg-surface-sunken text-ink-muted"
        >
          <KeyRound size={13} />
        </span>
        <div className="min-w-0 break-words text-[12px] leading-[1.6] text-ink-muted">
          <p className="font-medium text-ink">Where keys live</p>
          <p>
            Each key is a separate file under{" "}
            <code className="font-mono text-[11.5px]">{secretsDir}/</code> — directory 0700, file
            0600, replaced atomically. The index next to it holds labels and order only, never key
            material. Nothing here is copied into settings.json: Claude Code runs a helper script
            that cats the file.
          </p>
        </div>
      </section>
    </div>
  );
}
