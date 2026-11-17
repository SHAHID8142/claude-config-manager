import {
  useCallback,
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { Info } from "lucide-react";

export interface HelpPopoverProps {
  title: string;
  children: ReactNode;
  label?: string;
}

/** One information affordance for sections and fields. Native popovers escape
 * scrolling containers; older WebViews use the same keyboard/pointer fallback. */
export function HelpPopover({ title, children, label }: HelpPopoverProps) {
  const [open, setOpen] = useState(false);
  const id = useId();
  const trigger = useRef<HTMLButtonElement>(null);
  const panel = useRef<HTMLDivElement>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const native = typeof HTMLElement !== "undefined" && "showPopover" in HTMLElement.prototype;

  const close = useCallback(() => {
    if (timer.current) clearTimeout(timer.current);
    if (native && panel.current?.matches(":popover-open")) panel.current.hidePopover();
    setOpen(false);
  }, [native]);
  function show() {
    if (timer.current) clearTimeout(timer.current);
    setOpen(true);
    if (native && !panel.current?.matches(":popover-open")) panel.current?.showPopover();
  }
  function leave() {
    if (timer.current) clearTimeout(timer.current);
    timer.current = setTimeout(() => {
      if (
        document.activeElement !== trigger.current &&
        !panel.current?.contains(document.activeElement)
      )
        close();
    }, 160);
  }

  useLayoutEffect(() => {
    if (!open || !trigger.current || !panel.current) return;
    const place = () => {
      if (!trigger.current || !panel.current) return;
      const rect = trigger.current.getBoundingClientRect();
      const tip = panel.current;
      const width = Math.min(300, window.innerWidth - 24);
      tip.style.width = `${width}px`;
      tip.style.left = `${Math.max(12, Math.min(rect.left, window.innerWidth - width - 12))}px`;
      tip.style.top = `${Math.max(12, rect.bottom + tip.offsetHeight + 10 > window.innerHeight ? rect.top - tip.offsetHeight - 8 : rect.bottom + 8)}px`;
    };
    place();
    window.addEventListener("resize", place);
    window.addEventListener("scroll", place, true);
    return () => {
      window.removeEventListener("resize", place);
      window.removeEventListener("scroll", place, true);
    };
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const key = (event: KeyboardEvent) => {
      if (event.key === "Escape") close();
    };
    const outside = (event: PointerEvent) => {
      if (
        !trigger.current?.contains(event.target as Node) &&
        !panel.current?.contains(event.target as Node)
      )
        close();
    };
    document.addEventListener("keydown", key);
    document.addEventListener("pointerdown", outside);
    return () => {
      document.removeEventListener("keydown", key);
      document.removeEventListener("pointerdown", outside);
    };
  }, [open, close]);
  useEffect(
    () => () => {
      if (timer.current) clearTimeout(timer.current);
    },
    [],
  );

  return (
    <span className="inline-flex align-middle">
      <button
        ref={trigger}
        type="button"
        aria-label={label ?? `About ${title}`}
        aria-expanded={open}
        aria-controls={id}
        onMouseEnter={show}
        onMouseLeave={leave}
        onFocus={show}
        onBlur={leave}
        onClick={show}
        className="inline-grid size-6 shrink-0 place-items-center rounded-full text-ink-muted hover:bg-surface-sunken hover:text-ink focus-visible:outline-2 focus-visible:outline-accent"
      >
        <Info className="size-3.5" strokeWidth={1.8} aria-hidden="true" />
      </button>
      {(native || open) && (
        <div
          ref={panel}
          id={id}
          role="tooltip"
          aria-label={title}
          popover={native ? "auto" : undefined}
          onToggle={(event) => {
            if (native) setOpen((event.nativeEvent as ToggleEvent).newState === "open");
          }}
          onMouseEnter={() => {
            if (timer.current) clearTimeout(timer.current);
          }}
          onMouseLeave={leave}
          style={{
            position: "fixed",
            inset: "auto",
            margin: 0,
            display: !native && !open ? "none" : undefined,
            zIndex: 100,
          }}
          className="max-h-[min(28rem,80vh)] overflow-y-auto rounded-lg border border-line bg-surface p-3 text-left text-[12px] leading-relaxed font-normal text-ink-muted shadow-pop"
        >
          <p className="mb-1 font-semibold text-ink">{title}</p>
          {children}
        </div>
      )}
    </span>
  );
}
