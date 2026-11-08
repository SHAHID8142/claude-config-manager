import { useId, type SelectHTMLAttributes } from "react";
import { ChevronDown } from "lucide-react";

export interface SelectOption {
  value: string;
  label: string;
  disabled?: boolean;
}

export interface SelectFieldProps extends SelectHTMLAttributes<HTMLSelectElement> {
  label: string;
  hint?: string;
  options: ReadonlyArray<SelectOption>;
  placeholder?: string;
}

/** Native `<select>` (correct a11y and macOS behaviour) with a custom chevron. */
export function SelectField({
  label,
  hint,
  options,
  placeholder,
  className = "",
  id,
  ...rest
}: SelectFieldProps) {
  const auto = useId();
  const selectId = id ?? auto;
  const hintId = `${selectId}-hint`;

  return (
    <div className="flex flex-col gap-1">
      <label htmlFor={selectId} className="text-[12px] font-medium text-ink-muted">
        {label}
      </label>
      <div className="relative">
        <select
          {...rest}
          id={selectId}
          aria-describedby={hint ? hintId : undefined}
          className={`h-8 w-full appearance-none rounded-md border border-line bg-surface pr-8 pl-2.5 text-[13px] text-ink focus:border-accent disabled:bg-surface-muted disabled:text-ink-subtle ${className}`}
        >
          {placeholder !== undefined && (
            <option value="" disabled={rest.value !== ""}>
              {placeholder}
            </option>
          )}
          {options.map((o) => (
            <option key={o.value} value={o.value} disabled={o.disabled}>
              {o.label}
            </option>
          ))}
        </select>
        <ChevronDown
          aria-hidden="true"
          className="pointer-events-none absolute top-1/2 right-2.5 size-3.5 -translate-y-1/2 text-ink-subtle"
          strokeWidth={2}
        />
      </div>
      {hint && (
        <p id={hintId} className="text-[11.5px] leading-snug text-ink-subtle">
          {hint}
        </p>
      )}
    </div>
  );
}
