import type { ButtonHTMLAttributes, ReactNode } from "react";

export interface IconButtonProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children"> {
  /** Accessible name — icon-only buttons have no visible text. */
  label: string;
  icon: ReactNode;
  size?: "sm" | "md";
  tone?: "neutral" | "danger";
}

const SIZES = { sm: "size-7", md: "size-8" } as const;
const TONES = {
  neutral: "text-ink-muted hover:bg-surface-hover hover:text-ink",
  danger: "text-ink-muted hover:bg-danger-soft hover:text-danger",
} as const;

/** Square, icon-only control with a consistent hit area and focus ring. */
export function IconButton({
  label,
  icon,
  size = "md",
  tone = "neutral",
  className = "",
  type = "button",
  ...rest
}: IconButtonProps) {
  return (
    <button
      {...rest}
      type={type}
      aria-label={label}
      title={rest.title ?? label}
      className={`grid shrink-0 place-items-center rounded-md ${SIZES[size]} ${TONES[tone]} motion-safe:transition-colors disabled:cursor-not-allowed disabled:opacity-45 disabled:hover:bg-transparent ${className}`}
    >
      <span aria-hidden="true" className="grid place-items-center">
        {icon}
      </span>
    </button>
  );
}
