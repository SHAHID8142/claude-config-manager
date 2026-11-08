import { useId, type InputHTMLAttributes } from "react";

export interface TextInputProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "size"> {
  label: string;
  hint?: string;
  error?: string | null;
  mono?: boolean;
}

/** Labelled text field with hint/error wiring (`aria-describedby` + `aria-invalid`). */
export function TextInput({
  label,
  hint,
  error,
  mono,
  className = "",
  id,
  ...rest
}: TextInputProps) {
  const auto = useId();
  const inputId = id ?? auto;
  const hintId = `${inputId}-hint`;
  const errorId = `${inputId}-error`;
  const describedBy = [hint ? hintId : null, error ? errorId : null].filter(Boolean).join(" ");

  return (
    <div className="flex flex-col gap-1">
      <label htmlFor={inputId} className="text-[12px] font-medium text-ink-muted">
        {label}
      </label>
      <input
        {...rest}
        id={inputId}
        aria-invalid={error ? true : undefined}
        aria-describedby={describedBy || undefined}
        className={`h-8 w-full rounded-md border bg-surface px-2.5 text-[13px] text-ink placeholder:text-ink-subtle disabled:bg-surface-muted disabled:text-ink-subtle ${
          error ? "border-danger" : "border-line focus:border-accent"
        } ${mono ? "font-mono text-[12.5px]" : ""} ${className}`}
      />
      {hint && !error && (
        <p id={hintId} className="text-[11.5px] leading-snug text-ink-subtle">
          {hint}
        </p>
      )}
      {error && (
        <p id={errorId} role="alert" className="text-[11.5px] leading-snug text-danger">
          {error}
        </p>
      )}
    </div>
  );
}
