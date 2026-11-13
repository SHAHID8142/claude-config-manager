import { Gauge, KeyRound, Layers, Route, Sparkles, Wrench, type LucideIcon } from "lucide-react";
import { focusRing } from "../lib/focus";
import { TABS, type TabId } from "../lib/tab";

const ICONS: Record<TabId, LucideIcon> = {
  dashboard: Gauge,
  provider: Sparkles,
  credentials: KeyRound,
  model: Layers,
  routing: Route,
  maintenance: Wrench,
};

/** One number or word the sidebar can show beside a section, e.g. "3 keys". */
export type NavBadge = Partial<Record<TabId, string>>;

interface AppSidebarProps {
  active: TabId;
  onSelect: (tab: TabId) => void;
  badges?: NavBadge;
}

/**
 * macOS-style settings sidebar: one flat list, 30px rows, a quiet grey pill for
 * the current screen and the accent only on its icon. The nav recedes — the
 * content column is what the eye should land on.
 */
export function AppSidebar({ active, onSelect, badges = {} }: AppSidebarProps) {
  return (
    <nav aria-label="Sections" className="flex w-full shrink-0 flex-col gap-0.5 py-3 pr-3 pl-3">
      {TABS.map((tab) => {
        const Icon = ICONS[tab.id];
        const current = tab.id === active;
        const badge = badges[tab.id];
        return (
          <button
            key={tab.id}
            type="button"
            aria-current={current ? "page" : undefined}
            onClick={() => onSelect(tab.id)}
            className={`flex h-8 items-center gap-2.5 rounded-[6px] px-2.5 text-[13px] motion-safe:transition-colors ${focusRing} ${
              current
                ? "bg-surface font-medium text-ink shadow-[0_0_0_1px_var(--line)]"
                : "text-ink-muted hover:bg-surface-hover hover:text-ink"
            }`}
          >
            <Icon
              aria-hidden="true"
              size={15}
              strokeWidth={current ? 2 : 1.75}
              className={current ? "text-accent" : "text-ink-subtle"}
            />
            <span className="truncate">{tab.label}</span>
            {badge !== undefined && (
              // Decorative: the count must not become part of the button's name.
              <span
                aria-hidden="true"
                className="ml-auto shrink-0 text-[11px] tabular-nums text-ink-subtle"
              >
                {badge}
              </span>
            )}
          </button>
        );
      })}
    </nav>
  );
}
