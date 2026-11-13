/** Async-action button (T15, CTA system restyle): pending state with
 * aria-busy, disabled while pending, spinner respects prefers-reduced-motion.
 * Faces come from the shared .btn system in theme.css — gradient-lit
 * primary, gradient surface secondary — with the full state matrix
 * (hover / active / focus-visible / disabled / loading) defined there. */
import { focusRing } from "../lib/focus";

export interface LoadingButtonProps {
  label: string;
  pendingLabel?: string;
  pending?: boolean;
  disabled?: boolean;
  variant?: "primary" | "secondary" | "danger";
  /** Stable hook for demo recording / QA automation. */
  testId?: string;
  onClick?: () => void;
}

const variants = {
  primary: "btn-primary",
  secondary: "btn-secondary",
  danger: "btn-danger",
};

export function LoadingButton({
  label,
  pendingLabel,
  pending = false,
  disabled = false,
  variant = "secondary",
  testId,
  onClick,
}: LoadingButtonProps) {
  const inert = disabled || pending;
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={inert}
      aria-busy={pending}
      {...(testId !== undefined ? { "data-testid": testId } : {})}
      className={`btn ${variants[variant]} ${focusRing}`}
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
