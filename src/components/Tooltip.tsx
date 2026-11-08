import { useId, useRef, useState, type ReactNode } from "react";
import { Info } from "lucide-react";

export interface TooltipProps {
  /** Explanation shown on hover *and* keyboard focus. */
  content: ReactNode;
  /** Accessible name for the trigger (defaults to "More information"). */
  label?: string;
  className?: string;
}

/**
 * Info affordance. The trigger is a real button, so the text is reachable by
 * keyboard and announced through `aria-describedby` — hover alone is never the
 * only way to read it (V2-09 acceptance criterion).
 */
export function Tooltip({ content, label = "More information", className = "" }: TooltipProps) {
  const [open, setOpen] = useState(false);
  const id = useId();
  const timer = useRef<number | null>(null);

  function show() {
    if (timer.current !== null) window.clearTimeout(timer.current);
    setOpen(true);
  }
  function hideSoon() {
    timer.current = window.setTimeout(() => setOpen(false), 80);
  }

  return (
    <span className={`relative inline-flex align-middle ${className}`}>
      <button
        type="button"
        aria-label={label}
        aria-describedby={open ? id : undefined}
        onMouseEnter={show}
        onMouseLeave={hideSoon}
        onFocus={show}
        onBlur={() => setOpen(false)}
        onKeyDown={(e) => {
          if (e.key === "Escape") setOpen(false);
        }}
        className="grid size-4 place-items-center rounded-full text-ink-subtle hover:text-ink-muted focus-visible:outline-none"
      >
        <Info className="size-3.5" strokeWidth={1.9} aria-hidden="true" />
      </button>
      {open && (
        <span
          role="tooltip"
          id={id}
          onMouseEnter={show}
          onMouseLeave={hideSoon}
          className="absolute top-5 left-1/2 z-30 w-64 -translate-x-1/2 rounded-md border border-line bg-surface px-2.5 py-2 text-[12px] leading-snug font-normal text-ink-muted shadow-pop"
        >
          {content}
        </span>
      )}
    </span>
  );
}
