import {
  describeConnection,
  type ConnectionContext,
  type ConnectionState,
} from "../lib/connectionLabel";
import type { EffectiveStatus } from "../lib/ipc";

export interface AppHeaderProps {
  status: EffectiveStatus | null;
  /** V3-24: proxy liveness and the last explicit connection test. */
  connection?: ConnectionContext;
}

/** Dot colour follows the proven state, not the presence of a file. */
function connectionTone(state: ConnectionState): string {
  switch (state) {
    case "verified":
      return "bg-ok";
    case "configured":
      return "bg-ink-subtle";
    case "checking":
      return "bg-ink-subtle";
    case "unavailable":
    case "proxy-stopped":
    case "needs-attention":
      return "bg-warn";
    case "not-configured":
      return "bg-ink-subtle";
  }
}

/**
 * Title bar row: app identity on the left, the one live fact on the right.
 * Appearance lives at the foot of the sidebar, where macOS settings keeps it,
 * so nothing here fights with the window controls.
 */
export function AppHeader({ status, connection = {} }: AppHeaderProps) {
  const described = describeConnection(status, connection);
  return (
    <header
      data-tauri-drag-region
      className="flex h-12 shrink-0 items-center justify-between gap-3 border-b border-line px-5 select-none"
    >
      <div className="flex min-w-0 items-center gap-2.5">
        <img src="/app-icon.png" alt="" width={24} height={24} className="size-6 shrink-0" />
        <h1 className="truncate text-[13px] font-semibold tracking-[-0.005em] text-ink">
          Claude Config Manager
        </h1>
      </div>

      <span
        aria-label="Connection mode"
        className="inline-flex items-center gap-1.5 rounded-full border border-line bg-surface px-2.5 py-[3px] text-[11px] font-medium text-ink-muted"
      >
        <span
          aria-hidden="true"
          className={`size-1.5 rounded-full ${connectionTone(described.state)}`}
        />
        {`mode: ${described.label}`}
      </span>
    </header>
  );
}
