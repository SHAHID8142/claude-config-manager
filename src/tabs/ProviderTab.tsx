import { ApiKeyField } from "../components/ApiKeyField";
import { BridgeBadge, CapabilityBadge } from "../components/CapabilityBadge";
import { FieldRow } from "../components/FieldRow";
import { LoadingButton } from "../components/LoadingButton";
import { Section } from "../components/Section";
import { StatusBanner } from "../components/StatusBanner";
import { classifyEndpoint, endpointCheckMessage } from "../lib/endpoint";
import { focusRing } from "../lib/focus";
import type { ConnectionTestResult, ProviderSummary } from "../lib/ipc";

const inputClass = `h-9 w-full rounded-[6px] border border-control bg-surface px-2.5 text-[13px] text-ink placeholder:text-ink-subtle disabled:cursor-not-allowed disabled:opacity-50 ${focusRing}`;

export interface ProviderTabProps {
  providers: ProviderSummary[] | null;
  selectedId: string;
  onSelectProvider: (id: string) => void;
  selected: ProviderSummary | null;
  isCustom: boolean;
  customName: string;
  onCustomName: (value: string) => void;
  customEndpoint: string;
  onCustomEndpoint: (value: string) => void;
  apiKey: string;
  onApiKey: (value: string) => void;
  storedCredential: boolean;
  /** Sentinels from the shell; the tab never invents a provider id. */
  emptyId: string;
  customId: string;
  testing: boolean;
  testDisabled: boolean;
  onTestConnection: () => void;
  testResult: ConnectionTestResult | null;
  testError: string | null;
  onDismissTest: () => void;
  onDismissTestError: () => void;
}

/**
 * Provider screen: the source of every endpoint Claude Code uses, plus the key
 * that authenticates it. Connection test lives here because it proves *this*
 * provider and *this* key.
 */
export function ProviderTab(props: ProviderTabProps) {
  const { selected, isCustom, providers } = props;
  const endpointCheck =
    isCustom && props.customEndpoint !== "" ? classifyEndpoint(props.customEndpoint) : null;
  const endpointMessage = endpointCheck === null ? null : endpointCheckMessage(endpointCheck);
  const endpointIsError =
    endpointCheck !== null &&
    (endpointCheck.check === "invalid" || endpointCheck.check === "insecure");

  return (
    <section aria-label="Provider" className="flex flex-col gap-6">
      {props.testResult !== null && (
        <StatusBanner
          severity={props.testResult.ok ? "success" : "error"}
          message={
            props.testResult.ok
              ? `Connection OK — ${props.testResult.message} (${props.testResult.latencyMs} ms)`
              : `Connection failed — ${props.testResult.message}`
          }
          detail={props.testResult.endpoint}
          onDismiss={props.onDismissTest}
        />
      )}
      {props.testError !== null && (
        <StatusBanner
          severity="error"
          message="Connection test could not run."
          detail={props.testError}
          onDismiss={props.onDismissTestError}
        />
      )}

      <Section
        title="Service"
        description="Claude Code sends every request to this service. Verified presets cannot be edited; a custom provider is your own Anthropic-compatible endpoint."
      >
        <FieldRow
          label="Provider"
          htmlFor="provider-select"
          hint={
            providers === null
              ? "Loading provider catalog…"
              : "Custom providers use your own Anthropic-compatible endpoint."
          }
        >
          <select
            id="provider-select"
            value={props.selectedId}
            disabled={providers === null}
            onChange={(e) => props.onSelectProvider(e.target.value)}
            className={inputClass}
          >
            <option value={props.emptyId}>
              {providers === null ? "Loading…" : "Select a provider"}
            </option>
            {providers?.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
            {providers !== null && <option value={props.customId}>Custom…</option>}
          </select>
        </FieldRow>

        {selected !== null && (
          <div className="mb-2 flex flex-wrap items-center gap-2">
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
        )}

        {isCustom && (
          <>
            <FieldRow label="Custom provider name" htmlFor="custom-name-input" hint="Shown in the UI only.">
              <input
                id="custom-name-input"
                type="text"
                value={props.customName}
                onChange={(e) => props.onCustomName(e.target.value)}
                placeholder="My gateway"
                className={inputClass}
              />
            </FieldRow>
            <FieldRow
              label="Custom endpoint URL"
              htmlFor="custom-endpoint-input"
              hint="Must be an Anthropic-compatible https endpoint."
            >
              <input
                id="custom-endpoint-input"
                type="url"
                value={props.customEndpoint}
                onChange={(e) => props.onCustomEndpoint(e.target.value)}
                placeholder="https://"
                className={inputClass}
              />
            </FieldRow>
            {endpointMessage !== null && (
              <p
                aria-live="polite"
                id="custom-endpoint-check"
                className={`mb-2 text-[12px] ${endpointIsError ? "font-medium text-danger" : "text-ink-muted"}`}
              >
                {endpointMessage}
              </p>
            )}
          </>
        )}

        {selected !== null && !isCustom && selected.protocol !== "open_ai_only" && (
          <FieldRow
            label="Endpoint URL"
            htmlFor="endpoint-input"
            hint="Provided by the verified preset and not editable."
          >
            <input
              id="endpoint-input"
              type="url"
              disabled
              value={selected.protocol === "anthropic_native" ? "https://api.anthropic.com" : ""}
              readOnly
              className={inputClass}
            />
          </FieldRow>
        )}
      </Section>

      <Section
        title="Key"
        description="The key Claude Code authenticates with. It is stored as an owner-only file (0600) and handed over by an apiKeyHelper script."
      >
        <FieldRow
          label="API key"
          htmlFor="api-key-input"
          hint={
            selected === null && !isCustom
              ? "Enabled once a provider is selected."
              : props.storedCredential
                ? "A key is already stored for this provider — leave this blank to keep it and change only the model."
                : "Stored as an owner-only file (0600) in Application Support when saving. Never written to logs or backups in plaintext."
          }
        >
          <ApiKeyField
            id="api-key-input"
            value={props.apiKey}
            onChange={props.onApiKey}
            disabled={selected === null && !isCustom}
          />
        </FieldRow>
        <div className="mt-3 flex flex-wrap items-center gap-3">
          <LoadingButton
            label="Test Connection"
            pendingLabel="Testing…"
            pending={props.testing}
            disabled={props.testDisabled}
            variant="secondary"
            onClick={props.onTestConnection}
          />
          <p className="text-[12px] text-ink-muted">
            Sends one authenticated read to the provider. Nothing is written.
          </p>
        </div>
      </Section>
    </section>
  );
}
