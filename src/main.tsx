import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { ErrorBoundary } from "./components/ErrorBoundary";
import "./index.css";
import { applyTheme, loadTheme } from "./lib/theme";

// Apply the stored theme before first paint (V2-05). The toggle UI arrives in V2-16.
applyTheme(loadTheme());
globalThis.matchMedia?.("(prefers-color-scheme: dark)").addEventListener?.("change", () => {
  if (loadTheme() === "system") applyTheme("system");
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  </React.StrictMode>,
);
