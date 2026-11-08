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
  // Near-black, not the brand hue: the accent is kept for data and the one
  // selected mark (achromatic-ledger move).
  primary: "bg-primary text-on-primary hover:bg-primary-hover",
  secondary: "border border-control bg-surface text-ink hover:bg-surface-hover",
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
      className={`inline-flex h-8 items-center gap-2 rounded-[6px] px-3 text-[13px] font-medium motion-safe:transition-colors disabled:cursor-not-allowed disabled:opacity-50 ${variants[variant]} ${focusRing}`}
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
