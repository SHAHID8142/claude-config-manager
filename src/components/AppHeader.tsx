import { connectionLabel } from "../lib/connectionLabel";
import type { EffectiveStatus } from "../lib/ipc";

export interface AppHeaderProps {
  status: EffectiveStatus | null;
}

/** Dot colour for the state pill: ok / needs attention / nothing yet. */
function connectionTone(status: EffectiveStatus | null): string {
  if (status === null) return "bg-ink-subtle";
  if (!status.settingsFound) return "bg-ink-subtle";
  if (status.warnings.length > 0) return "bg-warn";
  return "bg-ok";
}

/**
 * Title bar row: app identity on the left, the one live fact on the right.
 * Appearance lives at the foot of the sidebar, where macOS settings keeps it,
 * so nothing here fights with the window controls.
 */
export function AppHeader({ status }: AppHeaderProps) {
  return (
    <header
      data-tauri-drag-region
      className="flex h-12 shrink-0 items-center justify-between gap-3 border-b border-line px-5 select-none"
    >
      <div className="flex min-w-0 items-center gap-2.5">
        <img
          src="/app-icon.png"
          alt=""
          width={22}
          height={22}
          className="size-[22px] shrink-0 rounded-[5px]"
        />
        <h1 className="truncate text-[13px] font-semibold tracking-[-0.005em] text-ink">
          Claude Config Manager
        </h1>
      </div>

      <span
        aria-label="Connection mode"
        className="inline-flex items-center gap-1.5 rounded-full border border-line bg-surface px-2.5 py-[3px] text-[11px] font-medium text-ink-muted"
      >
        <span aria-hidden="true" className={`size-1.5 rounded-full ${connectionTone(status)}`} />
        {`mode: ${connectionLabel(status)}`}
      </span>
    </header>
  );
}
