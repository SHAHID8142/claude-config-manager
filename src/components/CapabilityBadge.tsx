/** Capability and connection badges (T25, polished per DESIGN.md).
 * Labels are driven entirely by preset data; "unknown" is a first-class
 * state (PRD F12: no invented capability values).
 * Craft-floor compliance: drawn lucide icons (no glyphs), token-only colors
 * (ok/warn/accent soft surfaces), one hairline treatment, truthful copy. */
import { Check, CircleQuestionMark, Plug } from "lucide-react";
import { focusRing } from "../lib/focus";

export interface CapabilityBadgeProps {
  status: "verified" | "unknown";
  sourceUrl?: string | null;
}

const chip =
  "inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-[11.5px] font-medium leading-none";

export function CapabilityBadge({ status, sourceUrl }: CapabilityBadgeProps) {
  if (status === "verified") {
    return (
      <span
        className={`${chip} bg-ok-soft text-ok`}
        title="Verified against the provider's own documentation"
      >
        <Check size={11} strokeWidth={2.5} aria-hidden="true" />
        capabilities verified
        {sourceUrl != null && (
          <a
            href={sourceUrl}
            target="_blank"
            rel="noreferrer"
            className={`underline decoration-ok/40 underline-offset-2 hover:decoration-ok ${focusRing} rounded-[3px]`}
          >
            source
          </a>
        )}
      </span>
    );
  }
  return (
    <span
      className={`${chip} bg-surface-sunken text-ink-muted`}
      title="No verified capability data for this provider"
    >
      <CircleQuestionMark size={11} strokeWidth={2} aria-hidden="true" />
      capabilities unknown
    </span>
  );
}

export interface BridgeBadgeProps {
  required: boolean;
}

export function BridgeBadge({ required }: BridgeBadgeProps) {
  if (!required) {
    return (
      <span
        className={`${chip} bg-surface-sunken text-ink-muted`}
        title="Connects directly — no bridge process"
      >
        direct connection
      </span>
    );
  }
  return (
    <span className={`${chip} bg-warn-soft text-warn`} title="Set it up on the Failover screen">
      <Plug size={11} strokeWidth={2} aria-hidden="true" />
      bridge required — set up on Failover
    </span>
  );
}
