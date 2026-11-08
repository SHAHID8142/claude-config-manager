/** Masked API key entry with reveal/hide interaction (T20).
 * Security behaviors:
 * - Starts masked (`type="password"`), reveal is explicit and temporary.
 * - Auto-rehides after REVEAL_TIMEOUT_MS in case the user walks away.
 * - Re-hides immediately when the field loses focus.
 * - The value is held only in controlled React state; the component never
 *   logs, copies, or persists it (persistence is SecretStore's job, T21+).
 */
import { useEffect, useRef, useState } from "react";
import { focusRing } from "../lib/focus";

const REVEAL_TIMEOUT_MS = 30_000;

export interface ApiKeyFieldProps {
  id: string;
  value: string;
  onChange: (value: string) => void;
  disabled?: boolean;
  /** Hint linkage forwarded from FieldRow's aria-describedby merge. */
  "aria-describedby"?: string;
}

export function ApiKeyField({
  id,
  value,
  onChange,
  disabled = false,
  "aria-describedby": describedBy,
}: ApiKeyFieldProps) {
  const [revealed, setRevealed] = useState(false);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const clearTimer = () => {
    if (timerRef.current !== null) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
  };

  // While revealed, start the auto-rehide countdown; clear it on hide.
  useEffect(() => {
    if (revealed) {
      clearTimer();
      timerRef.current = setTimeout(() => setRevealed(false), REVEAL_TIMEOUT_MS);
    }
    return clearTimer;
  }, [revealed]);

  const toggleReveal = () => {
    setRevealed((r) => !r);
  };

  const hideOnBlur = () => {
    if (revealed) setRevealed(false);
  };

  return (
    <div className="flex items-center gap-2">
      <input
        id={id}
        type={revealed ? "text" : "password"}
        value={value}
        disabled={disabled}
        onChange={(e) => onChange(e.target.value)}
        onBlur={hideOnBlur}
        autoComplete="off"
        spellCheck={false}
        placeholder="••••••••••••"
        aria-describedby={describedBy}
        className={`w-full rounded-md border border-line bg-surface-raised px-3 py-2 font-mono text-sm text-ink-muted placeholder:text-ink-subtle disabled:cursor-not-allowed disabled:opacity-50 dark:border-line dark:bg-surface-raised dark:text-ink ${focusRing}`}
      />
      <button
        type="button"
        onClick={toggleReveal}
        disabled={disabled}
        aria-pressed={revealed}
        aria-controls={id}
        className={`shrink-0 rounded-md border border-line px-2 py-2 text-xs font-medium text-ink-muted disabled:cursor-not-allowed disabled:opacity-50 dark:border-line dark:text-ink ${focusRing}`}
      >
        {revealed ? "Hide" : "Show"}
      </button>
    </div>
  );
}
