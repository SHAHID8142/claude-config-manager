/**
 * T62 — keyboard, accessibility, macOS theme and small-window audit.
 *
 * Static/jsdom evidence only. A real macOS visual + VoiceOver pass is NOT
 * performed here and is disclosed as such in the audit document.
 */
import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import App from "./App";

import { StatusBanner } from "./components/StatusBanner";
import { createMockIpc } from "./lib/ipc-mock";
import { providerSummaryFixtures } from "./lib/fixtures/providers";

function renderApp() {
  const ipc = createMockIpc({
    handlers: {
      list_providers: () => Promise.resolve(providerSummaryFixtures),
    },
  });
  return render(<App ipc={ipc} />);
}

describe("document structure (T62)", () => {
  it("has exactly one h1 and never skips a heading level", async () => {
    renderApp();
    await screen.findByRole("heading", { level: 1 });
    const headings = screen.getAllByRole("heading");
    const levels = headings.map((h) => Number(h.tagName.slice(1)));
    expect(levels.filter((l) => l === 1)).toHaveLength(1);
    expect(levels[0]).toBe(1);
    for (let i = 1; i < levels.length; i += 1) {
      expect(levels[i] - levels[i - 1]).toBeLessThanOrEqual(1);
    }
  });

  it("gives every form control an accessible name", async () => {
    renderApp();
    await screen.findByLabelText("Provider", { selector: "select" });
    const controls = [
      ...screen.getAllByRole("textbox"),
      ...screen.getAllByRole("combobox"),
      ...screen.getAllByRole("button"),
    ];
    for (const el of controls) {
      // A native <label for> association is a valid accessible name; accept it
      // alongside aria-label/aria-labelledby and button text content.
      const labels = (el as HTMLInputElement).labels;
      const labelled = labels !== undefined && labels !== null && labels.length > 0;
      const name =
        el.getAttribute("aria-label") ?? el.getAttribute("aria-labelledby") ?? el.textContent ?? "";
      expect(labelled || name.trim().length > 0, `unnamed control: ${el.outerHTML}`).toBe(true);
    }
  });

  it("never uses a positive tabindex (focus order stays DOM order)", async () => {
    renderApp();
    await screen.findByLabelText("Provider", { selector: "select" });
    for (const el of document.querySelectorAll("[tabindex]")) {
      expect(Number(el.getAttribute("tabindex"))).toBeLessThanOrEqual(0);
    }
  });

  it("makes every key control keyboard-focusable in logical DOM order", async () => {
    renderApp();
    const provider = await screen.findByLabelText("Provider", { selector: "select" });
    // Credential and model fields are disabled until a provider is chosen, so
    // pick one first — an unreachable control is a bug, but an intentionally
    // disabled one is not.
    await userEvent.selectOptions(provider, "anthropic");
    const apiKey = screen.getByLabelText("API key", { selector: "input" });
    // A key enables Save & Apply; a disabled control cannot take focus (by
    // design), and this test is about *reachable* controls.
    await userEvent.type(apiKey, "sk-test-key");
    const model = screen.getByLabelText("Model", { selector: "input" });
    const fetch = screen.getByRole("button", { name: /fetch models/i });
    const apply = screen.getByRole("button", { name: /save & apply/i });

    // 1. Focusable: no control is removed from the tab sequence.
    for (const el of [provider, apiKey, model, fetch, apply]) {
      expect((el as HTMLElement).tabIndex).toBeGreaterThanOrEqual(0);
      el.focus();
      expect(document.activeElement).toBe(el);
    }

    // 2. DOM order matches the visual/task order: provider → credential →
    //    fetch (populate catalog) → model (choose) → apply.
    const chain = [provider, apiKey, fetch, model, apply];
    for (let i = 1; i < chain.length; i += 1) {
      const position = chain[i - 1].compareDocumentPosition(chain[i]);
      expect(
        position & Node.DOCUMENT_POSITION_FOLLOWING,
        `${chain[i - 1].outerHTML.slice(0, 40)} should precede ${chain[i].outerHTML.slice(0, 40)}`,
      ).toBeTruthy();
    }
  });
});

describe("announcements and decoration (T62)", () => {
  it("marks decorative disclosure glyphs as hidden from assistive tech", () => {
    renderApp();
    const toggle = screen.getByRole("button", { name: /advanced settings/i });
    const glyph = Array.from(toggle.querySelectorAll("span[aria-hidden='true']"));
    expect(glyph.length).toBeGreaterThan(0);
  });

  it("uses alert for errors and status for non-errors", () => {
    const { unmount } = render(<StatusBanner severity="error" message="boom" />);
    expect(screen.getByRole("alert")).toHaveAttribute("aria-live", "assertive");
    unmount();
    render(<StatusBanner severity="success" message="ok" />);
    expect(screen.getByRole("status")).toHaveAttribute("aria-live", "polite");
  });
});

describe("macOS theme and small-window resilience (T62, static)", () => {
  // Read the real shipped sources through Vite (no Node typings needed).
  const allTsx = import.meta.glob<string>("./**/*.tsx", {
    query: "?raw",
    import: "default",
    eager: true,
  });
  const sources: Array<[string, string]> = Object.entries(allTsx)
    .filter(([path]) => !path.includes(".test."))
    .map(([path, src]) => [path, src]);

  it("every surface that colours text also has a dark-variant colour", () => {
    for (const [file, src] of sources) {
      const hasLightText = /text-(zinc|red|green|amber|blue)-\d00/.test(src);
      if (!hasLightText) continue;
      expect(src, `${file} lacks dark: variants`).toMatch(/dark:text-/);
    }
  });

  it("uses no fixed pixel widths that would overflow a narrow window", () => {
    for (const [file, src] of sources) {
      expect(src, `${file} has a fixed pixel width`).not.toMatch(/\bw-\[\d+px\]/);
    }
  });

  it("wraps long unbreakable values instead of forcing horizontal scroll", () => {
    const app = sources.find(([p]) => p.endsWith("/App.tsx"))?.[1] ?? "";
    // Every <code> that renders a model id / URL / path must be breakable.
    const codeTags = app.match(/<code className="[^"]*"/g) ?? [];
    expect(codeTags.length).toBeGreaterThan(0);
    for (const tag of codeTags) {
      expect(tag).toContain("break-all");
    }
  });

  it("respects reduced motion (no animation/transition utilities without a guard)", () => {
    for (const [file, src] of sources) {
      // The app ships no motion; if one is added it must carry
      // motion-safe/motion-reduce so the macOS "Reduce motion" setting applies.
      const animated = /className="[^"]*\b(animate-|transition-)/.test(src);
      if (animated) {
        expect(src, `${file} animates without a reduced-motion guard`).toMatch(
          /motion-(safe|reduce)/,
        );
      }
    }
  });
});
