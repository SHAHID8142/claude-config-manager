/** Capability and connection badges (T25) — explicit, never fabricated.
 * Labels are driven entirely by preset data; "unknown" is a first-class
 * state (PRD F12: no invented capability values). */
import { focusRing } from "../lib/focus";

export interface CapabilityBadgeProps {
  status: "verified" | "unknown";
  sourceUrl?: string | null;
}

export function CapabilityBadge({ status, sourceUrl }: CapabilityBadgeProps) {
  const base = "inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-xs font-medium";
  if (status === "verified") {
    return (
      <span
        className={`${base} border-green-300 bg-green-50 text-green-800 dark:border-green-700 dark:bg-green-950 dark:text-green-200`}
      >
        <span aria-hidden="true">✓</span> capabilities verified
        {sourceUrl ? (
          <>
            {" "}
            <a
              href={sourceUrl}
              target="_blank"
              rel="noreferrer"
              className={`underline ${focusRing}`}
            >
              (source)
            </a>
          </>
        ) : null}
      </span>
    );
  }
  return (
    <span
      className={`${base} border-line bg-surface-sunken text-ink-muted dark:border-line dark:bg-surface-raised `}
    >
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
      <span className="inline-flex items-center gap-1 rounded-full border border-blue-300 bg-blue-50 px-2 py-0.5 text-xs font-medium text-blue-800 dark:border-blue-700 dark:bg-blue-950 dark:text-blue-200">
        <span aria-hidden="true">⇄</span> direct connection
      </span>
    );
  }
  return (
    <span className="inline-flex items-center gap-1 rounded-full border border-amber-300 bg-amber-50 px-2 py-0.5 text-xs font-medium text-amber-900 dark:border-amber-700 dark:bg-amber-950 dark:text-amber-200">
      <span aria-hidden="true">⚙</span> bridge required (arrives in a later milestone)
    </span>
  );
}
