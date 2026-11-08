import { focusRing } from "../lib/focus";

export interface SegmentedOption<T extends string> {
  value: T;
  label: string;
}

interface SegmentedControlProps<T extends string> {
  /** Accessible name for the group, e.g. "Appearance". */
  label: string;
  options: ReadonlyArray<SegmentedOption<T>>;
  value: T;
  onChange: (value: T) => void;
  disabled?: boolean;
}

/**
 * macOS segmented control: one inset track, equal segments, the selected one
 * raised onto a white pill. Used where 2–3 mutually exclusive choices belong in
 * the chrome rather than in a menu.
 */
export function SegmentedControl<T extends string>({
  label,
  options,
  value,
  onChange,
  disabled = false,
}: SegmentedControlProps<T>) {
  return (
    <div
      role="radiogroup"
      aria-label={label}
      className="flex gap-0.5 rounded-[7px] bg-surface-sunken p-0.5"
    >
      {options.map((option) => {
        const selected = option.value === value;
        return (
          <button
            key={option.value}
            type="button"
            role="radio"
            aria-checked={selected}
            disabled={disabled}
            onClick={() => onChange(option.value)}
            className={`flex h-6 flex-1 items-center justify-center rounded-[5px] text-[11.5px] motion-safe:transition-colors disabled:cursor-not-allowed disabled:opacity-50 ${focusRing} ${
              selected
                ? "bg-surface font-medium text-ink shadow-[0_0_0_1px_var(--line)]"
                : "text-ink-muted hover:text-ink"
            }`}
          >
            {option.label}
          </button>
        );
      })}
    </div>
  );
}
