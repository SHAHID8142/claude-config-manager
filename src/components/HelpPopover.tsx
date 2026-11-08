import { useEffect, useId, useRef, useState, type ReactNode } from "react";
import { CircleHelp } from "lucide-react";

export interface HelpPopoverProps {
  title: string;
  children: ReactNode;
  /** Trigger label, defaults to the title. */
  label?: string;
}

/**
 * Click/focus-triggered help panel for explanations too long for a tooltip.
 * Closes on Escape and on outside click; state is announced via aria-expanded.
 */
export function HelpPopover({ title, children, label }: HelpPopoverProps) {
  const [open, setOpen] = useState(false);
  const id = useId();
  const wrap = useRef<HTMLSpanElement>(null);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && setOpen(false);
    const onDown = (e: MouseEvent) => {
      if (wrap.current && !wrap.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("keydown", onKey);
    document.addEventListener("mousedown", onDown);
    return () => {
      document.removeEventListener("keydown", onKey);
      document.removeEventListener("mousedown", onDown);
    };
  }, [open]);

  return (
    <span className="relative inline-flex" ref={wrap}>
      <button
        type="button"
        aria-label={label ?? `About ${title}`}
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-controls={open ? id : undefined}
        onClick={() => setOpen((v) => !v)}
        className="grid size-4 place-items-center rounded-full text-ink-subtle hover:text-ink-muted focus-visible:outline-none"
      >
        <CircleHelp className="size-3.5" strokeWidth={1.9} aria-hidden="true" />
      </button>
      {open && (
        <div
          role="dialog"
          aria-label={title}
          id={id}
          className="absolute top-5 right-0 z-30 w-72 rounded-lg border border-line bg-surface p-3 text-[12px] leading-relaxed text-ink-muted shadow-pop"
        >
          <p className="mb-1 text-[12px] font-semibold text-ink">{title}</p>
          {children}
        </div>
      )}
    </span>
  );
}
