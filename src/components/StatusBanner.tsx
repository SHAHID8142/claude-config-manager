/** Feedback banner (T15): info / success / warning / error states.
 * Errors use role="alert" (assertive), everything else role="status"
 * (polite) so screen readers announce without interrupting. */

export type FeedbackSeverity = "info" | "success" | "warning" | "error";

export interface StatusBannerProps {
  severity: FeedbackSeverity;
  message: string;
  /** Optional remediation or detail line under the main message. */
  detail?: string;
  onDismiss?: () => void;
}

const severityStyles: Record<FeedbackSeverity, string> = {
  info: "border-blue-300 bg-blue-50 text-blue-900 dark:border-blue-700 dark:bg-blue-950 dark:text-blue-100",
  success:
    "border-green-300 bg-green-50 text-green-900 dark:border-green-700 dark:bg-green-950 dark:text-green-100",
  warning:
    "border-amber-300 bg-amber-50 text-amber-900 dark:border-amber-700 dark:bg-amber-950 dark:text-amber-100",
  error:
    "border-red-300 bg-red-50 text-red-900 dark:border-red-700 dark:bg-red-950 dark:text-red-100",
};

const severityLabel: Record<FeedbackSeverity, string> = {
  info: "Information",
  success: "Success",
  warning: "Warning",
  error: "Error",
};

export function StatusBanner({ severity, message, detail, onDismiss }: StatusBannerProps) {
  const isError = severity === "error";
  return (
    <div
      role={isError ? "alert" : "status"}
      aria-live={isError ? "assertive" : "polite"}
      className={`flex items-start gap-2 rounded-md border p-3 text-sm ${severityStyles[severity]}`}
    >
      <span className="sr-only">{`${severityLabel[severity]}: `}</span>
      <div className="flex-1">
        <p className="font-medium">{message}</p>
        {detail !== undefined && <p className="mt-0.5 text-xs opacity-80">{detail}</p>}
      </div>
      {onDismiss !== undefined && (
        <button
          type="button"
          onClick={onDismiss}
          aria-label="Dismiss message"
          className="rounded px-1 text-sm opacity-70 hover:opacity-100"
        >
          ✕
        </button>
      )}
    </div>
  );
}
