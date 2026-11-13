import { Info } from "lucide-react";
import { AdvancedSettings } from "../components/AdvancedSettings";
import { FieldRow } from "../components/FieldRow";
import { LoadingButton } from "../components/LoadingButton";
import { ModelList } from "../components/ModelList";
import { Section } from "../components/Section";
import { StatusBanner } from "../components/StatusBanner";
import { contextWindowLabel } from "../lib/statusText";
import { focusRing } from "../lib/focus";
import type { RoleKey, RoleState } from "../lib/roles";
import type { EffectiveStatus, ModelEntry, ProviderSummary } from "../lib/ipc";

const inputClass = `h-9 w-full rounded-[6px] border border-control bg-surface px-2.5 text-[13px] text-ink placeholder:text-ink-subtle disabled:cursor-not-allowed disabled:opacity-50 ${focusRing}`;

export interface ModelTabProps {
  provider: ProviderSummary | null;
  isCustom: boolean;
  models: ModelEntry[] | null;
  fetching: boolean;
  fetchedFor: string | null;
  onFetch: () => void;
  modelsError: string | null;
  onDismissModelsError: () => void;
  selectedModel: string | null;
  onSelectModel: (id: string) => void;
  modelInvalid: boolean;
  modelMessage: string | null;
  status: EffectiveStatus | null;
  roles: RoleState;
  onRole: (key: RoleKey, value: string) => void;
  roleInvalid: boolean;
  advancedOpen: boolean;
  onToggleAdvanced: () => void;
  contextWindow: string;
  onContextWindow: (value: string) => void;
}

/**
 * Model screen: what Claude Code asks for, what it is allowed to ask for, and
 * what Claude Code assumes about it. The applied value and the draft are shown
 * separately so a half-typed id is never mistaken for the live one.
 */
export function ModelTab(props: ModelTabProps) {
  const draft = props.selectedModel?.trim() ?? "";
  const applied = props.status?.envModel ?? props.status?.model ?? null;
  const unrecognized =
    draft !== "" &&
    !draft.toLowerCase().startsWith("claude") &&
    props.status?.envMaxContextTokens == null;

  return (
    <section aria-label="Model" className="flex flex-col gap-6">
      {/* The answer to "which model is selected": applied vs draft, side by side. */}
      <dl className="grid grid-cols-2 gap-4 rounded-[10px] border border-line bg-surface p-4">
        <div className="min-w-0 truncate">
          <dt className="text-[11px] text-ink-subtle">Applied (in Claude Code now)</dt>
          <dd className="mt-1 truncate font-mono text-[13px] text-ink" title={applied ?? undefined}>
            {applied ?? "not set"}
          </dd>
          <dd className="mt-0.5 text-[11px] text-ink-muted">
            Window {contextWindowLabel(props.status?.envMaxContextTokens ?? null)}
          </dd>
        </div>
        <div className="min-w-0 truncate">
          <dt className="text-[11px] text-ink-subtle">Draft (until you Save &amp; Apply)</dt>
          <dd
            className={`mt-1 truncate font-mono text-[13px] ${draft === "" ? "text-ink-muted" : "text-ink"}`}
            title={draft === "" ? undefined : draft}
          >
            {draft === "" ? "nothing chosen" : draft}
          </dd>
          <dd className="mt-0.5 text-[11px] text-ink-muted">
            {props.fetchedFor === null
              ? "manual entry"
              : `catalog for ${props.provider?.name ?? "provider"}`}
          </dd>
        </div>
      </dl>

      {props.modelsError !== null && (
        <StatusBanner
          severity="error"
          message="Model discovery failed."
          detail={props.modelsError}
          onDismiss={props.onDismissModelsError}
        />
      )}

      <Section title="Choose" description="Fetch the provider's catalog, then pick one.">
        <div className="mb-3 flex flex-wrap items-center gap-2">
          <LoadingButton
            label="Fetch models"
            pendingLabel="Fetching…"
            pending={props.fetching}
            disabled={props.provider === null || props.isCustom}
            variant="secondary"
            onClick={props.onFetch}
          />
          {props.provider !== null && !props.isCustom && (
            <span className="text-[12px] text-ink-subtle">
              Verified preset for {props.provider.name}
            </span>
          )}
        </div>

        {props.fetchedFor !== null &&
          (props.models !== null && props.models.length > 0 ? (
            <div className="mb-3">
              <ModelList
                models={props.models}
                selectedId={props.selectedModel}
                onSelect={props.onSelectModel}
              />
            </div>
          ) : (
            <p className="mb-3 text-[12px] text-ink-muted">
              Catalog returned no models — enter an id below.
            </p>
          ))}

        <FieldRow
          label="Model"
          htmlFor="model-input"
          hint="Exact id the provider expects. Never auto-selected."
        >
          <input
            id="model-input"
            type="text"
            value={props.selectedModel ?? ""}
            readOnly={props.models !== null && props.models.length > 0}
            disabled={props.provider === null && !props.isCustom}
            onChange={(e) => props.onSelectModel(e.target.value)}
            placeholder="provider-model-id"
            aria-invalid={props.modelInvalid}
            className={inputClass}
          />
        </FieldRow>

        {props.modelInvalid && props.modelMessage !== null && (
          <p
            aria-live="polite"
            id="model-input-error"
            className="text-[12px] font-medium text-danger"
          >
            {props.modelMessage} Save &amp; Apply is disabled until this is fixed.
          </p>
        )}

        {props.fetchedFor === null && props.provider !== null && (
          <p className="mt-1 text-[11px] text-ink-subtle">
            Source: manual entry · capability data unverified.
          </p>
        )}
      </Section>

      <Section
        title="Declared window"
        description="Declare the real window when a model is larger than 200k."
        help={{
          title: "About the context window",
          children: (
            <p>
              Claude Code assumes 200k for model ids it does not recognise. Declaring the real
              window keeps long sessions from being cut short.
            </p>
          ),
        }}
      >
        {unrecognized && (
          <p className="mb-3 flex items-start gap-2 rounded-[6px] bg-warn-soft px-3 py-2 text-[12px] leading-[1.5] text-warn">
            <Info aria-hidden="true" size={13} className="mt-0.5 shrink-0" />
            <span>
              This id is not one Claude Code knows, so it will assume a 200k window.{" "}
              <button
                type="button"
                className="underline underline-offset-2 hover:text-ink"
                onClick={props.onToggleAdvanced}
              >
                Declare its real window
              </button>{" "}
              below.
            </span>
          </p>
        )}
        <AdvancedSettings
          open={props.advancedOpen}
          onToggle={props.onToggleAdvanced}
          roles={props.roles}
          onChange={props.onRole}
          current={{
            haiku: props.status?.envDefaultHaiku ?? null,
            sonnet: props.status?.envDefaultSonnet ?? null,
            opus: props.status?.envDefaultOpus ?? null,
          }}
          contextWindow={props.contextWindow}
          onContextWindowChange={props.onContextWindow}
          currentContextWindow={props.status?.envMaxContextTokens ?? null}
        />
        {props.roleInvalid && (
          <p aria-live="polite" className="mt-2 text-[12px] font-medium text-danger">
            A role override is invalid. Save &amp; Apply is disabled until it is fixed.
          </p>
        )}
      </Section>
    </section>
  );
}
