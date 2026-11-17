/**
 * The app's sections, in the order the sidebar shows them. One id per screen:
 * the shell renders exactly one of these at a time, so a screen can be read
 * top-to-bottom instead of scrolled past.
 */
export const TABS = [
  { id: "dashboard", label: "Overview" },
  { id: "provider", label: "Providers & keys" },
  { id: "routing", label: "Switching" },
  { id: "maintenance", label: "Recovery" },
] as const;

export type TabId = (typeof TABS)[number]["id"] | "credentials" | "model";

export function isTabId(value: string): value is TabId {
  return TABS.some((t) => t.id === value);
}

/** Heading shown at the top of each screen: the name, then one line of purpose. */
export const TAB_META: Record<TabId, { title: string; purpose: string }> = {
  dashboard: {
    title: "Overview",
    purpose: "Saved global settings and any problems that need attention.",
  },
  provider: {
    title: "Providers & keys",
    purpose: "Choose a service, add its keys, choose a model, then add it to your switching order.",
  },
  credentials: {
    title: "Credentials",
    purpose: "Stored keys in priority order. Activation determines which key Claude Code uses.",
  },
  model: {
    title: "Model",
    purpose: "The model Claude Code requests, its context window, and role overrides.",
  },
  routing: {
    title: "Automatic switching",
    purpose: "Try each provider’s models and keys in order, then move to the next provider.",
  },
  maintenance: {
    title: "Recovery & diagnostics",
    purpose: "Backups, restores and the checks behind the status on the Dashboard.",
  },
};
