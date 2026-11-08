/**
 * The app's sections, in the order the sidebar shows them. One id per screen:
 * the shell renders exactly one of these at a time, so a screen can be read
 * top-to-bottom instead of scrolled past.
 */
export const TABS = [
  { id: "dashboard", label: "Dashboard" },
  { id: "provider", label: "Provider" },
  { id: "credentials", label: "Credentials" },
  { id: "model", label: "Model" },
  { id: "routing", label: "Failover" },
  { id: "maintenance", label: "Maintenance" },
] as const;

export type TabId = (typeof TABS)[number]["id"];

export function isTabId(value: string): value is TabId {
  return TABS.some((t) => t.id === value);
}

/** Heading shown at the top of each screen: the name, then one line of purpose. */
export const TAB_META: Record<TabId, { title: string; purpose: string }> = {
  dashboard: {
    title: "Dashboard",
    purpose: "What Claude Code is using right now, and what needs attention.",
  },
  provider: {
    title: "Provider",
    purpose: "Which service Claude Code talks to, and the key it authenticates with.",
  },
  credentials: {
    title: "Credentials",
    purpose: "Every key stored for this provider. The first enabled one is used.",
  },
  model: {
    title: "Model",
    purpose: "The model Claude Code requests, its context window, and role overrides.",
  },
  routing: {
    title: "Failover",
    purpose: "An optional local proxy that tries your targets in order when one fails.",
  },
  maintenance: {
    title: "Maintenance",
    purpose: "Backups, restores and the checks behind the status on the Dashboard.",
  },
};
