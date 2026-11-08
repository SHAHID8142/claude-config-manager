/** Async-action button (T15): pending state with aria-busy, disabled while
 * pending, spinner respects prefers-reduced-motion. */
import { focusRing } from "../lib/focus";

export interface LoadingButtonProps {
  label: string;
  pendingLabel?: string;
  pending?: boolean;
  disabled?: boolean;
  variant?: "primary" | "secondary";
  onClick?: () => void;
}

const variants = {
  primary: "bg-blue-600 text-white",
  secondary: "border border-line text-ink-muted dark:border-line dark:text-ink",
};

export function LoadingButton({
  label,
  pendingLabel,
  pending = false,
  disabled = false,
  variant = "secondary",
  onClick,
}: LoadingButtonProps) {
  const inert = disabled || pending;
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={inert}
      aria-busy={pending}
      className={`inline-flex items-center gap-2 rounded-md px-3 py-1.5 text-sm font-medium disabled:cursor-not-allowed disabled:opacity-50 ${variants[variant]} ${focusRing}`}
    >
      {pending && (
        <span
          aria-hidden="true"
          className="h-3 w-3 animate-spin rounded-full border-2 border-current border-t-transparent motion-reduce:animate-none motion-reduce:border-t-current"
        />
      )}
      {pending ? (pendingLabel ?? "Working…") : label}
    </button>
  );
}
