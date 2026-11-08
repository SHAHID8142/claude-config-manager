import type { ButtonHTMLAttributes, ReactNode } from "react";

export type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";
export type ButtonSize = "sm" | "md";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  icon?: ReactNode;
  block?: boolean;
}

const VARIANT: Record<ButtonVariant, string> = {
  primary:
    "bg-accent text-on-accent hover:bg-accent-hover border border-transparent shadow-[0_1px_2px_rgb(0_0_0/0.08)]",
  secondary: "bg-surface text-ink border border-line hover:bg-surface-hover",
  ghost:
    "bg-transparent text-ink-muted border border-transparent hover:bg-surface-hover hover:text-ink",
  danger: "bg-danger text-white border border-transparent hover:opacity-90",
};

const SIZE: Record<ButtonSize, string> = {
  sm: "h-7 px-2.5 text-[12px] gap-1.5 rounded-sm",
  md: "h-8 px-3 text-[13px] gap-1.5 rounded-md",
};

export function Button({
  variant = "secondary",
  size = "md",
  icon,
  block = false,
  className = "",
  type = "button",
  children,
  ...rest
}: ButtonProps) {
  return (
    <button
      {...rest}
      type={type}
      className={`inline-flex items-center justify-center font-medium whitespace-nowrap motion-safe:transition-colors disabled:cursor-not-allowed disabled:opacity-45 ${VARIANT[variant]} ${SIZE[size]} ${block ? "w-full" : ""} ${className}`}
    >
      {icon !== undefined && (
        <span aria-hidden="true" className="grid place-items-center">
          {icon}
        </span>
      )}
      {children}
    </button>
  );
}
