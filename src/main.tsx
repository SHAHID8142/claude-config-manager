import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { ErrorBoundary } from "./components/ErrorBoundary";
import "./index.css";
import { applyTheme, loadTheme } from "./lib/theme";
import { createDevMockIpc } from "./dev/mockIpc";

// Apply the stored theme before first paint (V2-05). The toggle UI arrives in V2-16.
applyTheme(loadTheme());
globalThis.matchMedia?.("(prefers-color-scheme: dark)").addEventListener?.("change", () => {
  if (loadTheme() === "system") applyTheme("system");
});

// Dev-only visual-audit harness: `npm run dev` + `?mock=1` renders the full
// app against fixture data in a plain browser. Production bundles never
// include this branch (import.meta.env.DEV is statically replaced).
const devIpc =
  import.meta.env.DEV && new URLSearchParams(window.location.search).has("mock")
    ? createDevMockIpc()
    : undefined;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ErrorBoundary>
      <App ipc={devIpc} />
    </ErrorBoundary>
  </React.StrictMode>,
);
