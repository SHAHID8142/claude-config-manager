import type { ButtonHTMLAttributes, ReactNode } from "react";

export type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";
export type ButtonSize = "sm" | "md";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  icon?: ReactNode;
  block?: boolean;
}

// Faces come from the shared .btn system in theme.css (gradient-lit,
// full state matrix). Sizes stay Tailwind here.
const VARIANT: Record<ButtonVariant, string> = {
  primary: "btn-primary",
  secondary: "btn-secondary",
  ghost: "btn-ghost",
  danger: "btn-danger",
};

const SIZE: Record<ButtonSize, string> = {
  sm: "!h-7 !px-2.5 !text-[12px] !gap-1.5 !rounded-sm",
  md: "!h-8 !px-3 !text-[13px] !gap-1.5 !rounded-md",
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
      className={`btn items-center justify-center ${VARIANT[variant]} ${SIZE[size]} ${block ? "w-full" : ""} ${className}`}
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
