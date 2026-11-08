import { useId } from "react";

export interface SwitchProps {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  hint?: string;
  disabled?: boolean;
}

/** `role="switch"` toggle: Space/Enter activate it (native button semantics). */
export function Switch({ label, checked, onChange, hint, disabled }: SwitchProps) {
  const hintId = useId();
  return (
    <div className="flex items-start justify-between gap-3">
      <div className="min-w-0">
        <p className="text-[13px] font-medium text-ink">{label}</p>
        {hint && (
          <p id={hintId} className="text-[11.5px] leading-snug text-ink-subtle">
            {hint}
          </p>
        )}
      </div>
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        aria-label={label}
        aria-describedby={hint ? hintId : undefined}
        disabled={disabled}
        onClick={() => onChange(!checked)}
        className={`relative mt-0.5 h-[22px] w-[38px] shrink-0 rounded-full border motion-safe:transition-colors disabled:opacity-45 ${
          checked ? "border-transparent bg-accent" : "border-line bg-surface-sunken"
        }`}
      >
        <span
          aria-hidden="true"
          className={`absolute top-[2px] size-[16px] rounded-full bg-white shadow-sm motion-safe:transition-[left] ${
            checked ? "left-[19px]" : "left-[2px]"
          }`}
        />
      </button>
    </div>
  );
}
