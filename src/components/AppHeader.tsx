import { useEffect, useRef, useState } from "react";
import { Check, Settings2 } from "lucide-react";
import { IconButton } from "./IconButton";
import { connectionLabel } from "../lib/connectionLabel";
import type { EffectiveStatus } from "../lib/ipc";
import { applyTheme, loadTheme, saveTheme, type ThemeMode } from "../lib/theme";

export interface AppHeaderProps {
  status: EffectiveStatus | null;
  /** Injected in tests so the popover can be asserted without touching storage. */
  themeStorage?: Pick<Storage, "getItem" | "setItem"> | null;
}

const MODES: ReadonlyArray<{ value: ThemeMode; label: string }> = [
  { value: "system", label: "System" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
];

/** Dot colour for the header pill: ok / needs attention / nothing yet. */
function connectionTone(status: EffectiveStatus | null): string {
  if (status === null) return "bg-ink-subtle";
  if (!status.settingsFound) return "bg-ink-subtle";
  if (status.warnings.length > 0) return "bg-warn";
  return "bg-ok";
}

/** Compact app chrome: icon, title, live state, appearance menu. */
export function AppHeader({ status, themeStorage }: AppHeaderProps) {
  const [open, setOpen] = useState(false);
  const [mode, setMode] = useState<ThemeMode>(() => loadTheme(themeStorage ?? undefined));
  const wrapRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    const onPointer = (e: MouseEvent) => {
      if (wrapRef.current && !wrapRef.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("keydown", onKey);
    document.addEventListener("mousedown", onPointer);
    return () => {
      document.removeEventListener("keydown", onKey);
      document.removeEventListener("mousedown", onPointer);
    };
  }, [open]);

  function choose(next: ThemeMode) {
    setMode(next);
    saveTheme(next, themeStorage ?? undefined);
    applyTheme(next);
  }

  return (
    <header
      data-tauri-drag-region
      className="flex h-12 items-center justify-between gap-3 pl-16 select-none"
    >
      <div className="flex min-w-0 items-center gap-2.5">
        <img
          src="/app-icon.png"
          alt=""
          width={24}
          height={24}
          className="size-6 shrink-0 rounded-[6px]"
        />
        <h1 className="truncate text-[15px] font-semibold tracking-[-0.01em] text-ink">
          Claude Config Manager
        </h1>
      </div>

      <div className="flex items-center gap-1.5">
        <span
          aria-label="Connection mode"
          className="inline-flex items-center gap-1.5 rounded-full border border-line bg-surface px-2.5 py-1 text-[11px] font-medium text-ink-muted"
        >
          <span aria-hidden="true" className={`size-1.5 rounded-full ${connectionTone(status)}`} />
          {`mode: ${connectionLabel(status)}`}
        </span>

        <div className="relative" ref={wrapRef}>
          <IconButton
            label="Settings"
            size="sm"
            aria-haspopup="dialog"
            aria-expanded={open}
            onClick={() => setOpen((v) => !v)}
            icon={<Settings2 className="size-4" strokeWidth={1.8} />}
          />

          {open && (
            <div
              role="dialog"
              aria-label="Appearance"
              className="absolute top-9 right-0 z-20 w-44 rounded-lg border border-line bg-surface p-1.5 shadow-pop"
            >
              <p className="px-2 py-1 text-[11px] font-medium tracking-wide text-ink-subtle uppercase">
                Appearance
              </p>
              <div role="radiogroup" aria-label="Theme">
                {MODES.map((m) => (
                  <button
                    key={m.value}
                    type="button"
                    role="radio"
                    aria-checked={mode === m.value}
                    onClick={() => choose(m.value)}
                    className={`flex w-full items-center justify-between rounded-md px-2 py-1.5 text-left text-[13px] motion-safe:transition-colors hover:bg-surface-hover ${
                      mode === m.value ? "text-ink" : "text-ink-muted"
                    }`}
                  >
                    {m.label}
                    {mode === m.value && (
                      <Check
                        className="size-3.5 text-accent"
                        strokeWidth={2.6}
                        aria-hidden="true"
                      />
                    )}
                  </button>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>
    </header>
  );
}
