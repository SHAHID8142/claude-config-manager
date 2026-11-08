/** Feedback banner (T15, restyled V2-14): info / success / warning / error.
 * Errors use role="alert" (assertive), everything else role="status" (polite)
 * so screen readers announce without interrupting. Tones are flat surfaces
 * with a single accent edge — no saturated pastel blocks, no glows. */
import { AlertTriangle, Check, Info, X, XCircle } from "lucide-react";

export type FeedbackSeverity = "info" | "success" | "warning" | "error";

export interface StatusBannerProps {
  severity: FeedbackSeverity;
  message: string;
  /** Optional remediation or detail line under the main message. */
  detail?: string;
  onDismiss?: () => void;
}

const severityStyles: Record<FeedbackSeverity, string> = {
  info: "border-line bg-surface-sunken text-ink",
  success: "border-ok/25 bg-ok-soft text-ink",
  warning: "border-warn/30 bg-warn-soft text-ink",
  error: "border-danger/30 bg-danger-soft text-ink",
};

const iconStyles: Record<FeedbackSeverity, string> = {
  info: "text-ink-subtle",
  success: "text-ok",
  warning: "text-warn",
  error: "text-danger",
};

const severityLabel: Record<FeedbackSeverity, string> = {
  info: "Information",
  success: "Success",
  warning: "Warning",
  error: "Error",
};

const Icon = { info: Info, success: Check, warning: AlertTriangle, error: XCircle };

export function StatusBanner({ severity, message, detail, onDismiss }: StatusBannerProps) {
  const isError = severity === "error";
  const Glyph = Icon[severity];
  return (
    <div
      role={isError ? "alert" : "status"}
      aria-live={isError ? "assertive" : "polite"}
      className={`flex items-start gap-2.5 rounded-lg border px-3 py-2.5 text-[12.5px] ${severityStyles[severity]}`}
    >
      <Glyph aria-hidden="true" className={`mt-0.5 size-3.5 shrink-0 ${iconStyles[severity]}`} />
      <span className="sr-only">{`${severityLabel[severity]}: `}</span>
      <div className="min-w-0 flex-1">
        <p className="font-medium text-ink">{message}</p>
        {detail !== undefined && (
          <p className="mt-0.5 text-[11.5px] break-words text-ink-muted">{detail}</p>
        )}
      </div>
      {onDismiss !== undefined && (
        <button
          type="button"
          onClick={onDismiss}
          aria-label="Dismiss message"
          className="rounded p-0.5 text-ink-subtle hover:text-ink"
        >
          <X aria-hidden="true" className="size-3.5" />
        </button>
      )}
    </div>
  );
}
