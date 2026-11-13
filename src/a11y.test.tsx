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
import { RoutingPanel } from "./components/RoutingPanel";
import { createMockIpc } from "./lib/ipc-mock";
import { providerSummaryFixtures } from "./lib/fixtures/providers";

/** The sidebar is a macOS settings list: one screen at a time. */
async function goTo(label: string) {
  await userEvent.click(screen.getByRole("button", { name: label }));
}

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
    await goTo("Provider");
    await screen.findByLabelText("Provider", { selector: "select" });
    // Not every screen has every kind of control; the invariant is about the
    // ones that are actually rendered.
    const controls = [
      ...screen.queryAllByRole("textbox"),
      ...screen.queryAllByRole("combobox"),
      ...screen.queryAllByRole("button"),
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
    await goTo("Provider");
    await screen.findByLabelText("Provider", { selector: "select" });
    for (const el of document.querySelectorAll("[tabindex]")) {
      expect(Number(el.getAttribute("tabindex"))).toBeLessThanOrEqual(0);
    }
  });

  it("makes every key control keyboard-focusable in logical DOM order", async () => {
    renderApp();
    await goTo("Provider");
    const provider = await screen.findByLabelText("Provider", { selector: "select" });
    // Credential and model fields are disabled until a provider is chosen, so
    // pick one first — an unreachable control is a bug, but an intentionally
    // disabled one is not.
    await userEvent.selectOptions(provider, "anthropic");
    const apiKey = screen.getByLabelText("API key", { selector: "input" });
    // A key enables Save & Apply; a disabled control cannot take focus (by
    // design), and this test is about *reachable* controls.
    await userEvent.type(apiKey, "sk-test-key");
    const apply = screen.getByRole("button", { name: /save & apply/i });

    // 1. Focusable: no control is removed from the tab sequence.
    for (const el of [provider, apiKey, apply]) {
      expect((el as HTMLElement).tabIndex).toBeGreaterThanOrEqual(0);
      el.focus();
      expect(document.activeElement).toBe(el);
    }

    // 2. DOM order matches the task order on this screen: provider → key.
    expect(
      provider.compareDocumentPosition(apiKey) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();

    // 3. The model screen keeps its own order: catalog first, then the field,
    //    and the primary action is still after the panel.
    await goTo("Model");
    const fetch = screen.getByRole("button", { name: /fetch models/i });
    const model = screen.getByLabelText("Model", { selector: "input" });
    for (const el of [fetch, model, apply]) {
      expect((el as HTMLElement).tabIndex).toBeGreaterThanOrEqual(0);
      el.focus();
      expect(document.activeElement).toBe(el);
    }
    expect(fetch.compareDocumentPosition(model) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });
});

describe("announcements and decoration (T62)", () => {
  it("marks decorative disclosure glyphs as hidden from assistive tech", async () => {
    renderApp();
    await goTo("Model");
    const toggle = screen.getByRole("button", { name: /advanced settings/i });
    const glyph = Array.from(toggle.querySelectorAll("[aria-hidden='true']"));
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

  it("never lets a long model id, URL or path force horizontal scroll", () => {
    // V2 shows long values in truncated rows (with the full value in the title
    // attribute), so the invariant is "truncate or break", not "always <code>".
    // Long values live on the Dashboard and the tab screens, not the shell.
    const app = sources
      .filter(([p]) => p.endsWith("/App.tsx") || p.includes("/tabs/"))
      .map(([, src]) => src)
      .join("\n");
    const longValueRows = app.match(/className="[^"]*\bmin-w-0\b[^"]*"/g) ?? [];
    expect(longValueRows.length).toBeGreaterThan(0);
    for (const row of longValueRows) {
      expect(row, "long values must truncate or break inside a min-w-0 box").toMatch(
        /truncate|break-(all|words)/,
      );
    }
    const nowrap = app.match(/className="[^"]*whitespace-nowrap[^"]*"/g) ?? [];
    for (const cls of nowrap) {
      expect(cls).toContain("truncate");
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

// V2-46: the failover routing surface follows the same rules as the rest of the
// app — labelled controls, real disabled states, no positive tabindex, and
// failure reasons as text rather than hover-only tooltips.
describe("failover routing accessibility (V2-46)", () => {
  const providers = [
    {
      id: "openrouter",
      name: "OpenRouter",
      protocol: "anthropic_compat" as const,
      bridge: "not_required" as const,
      capabilityStatus: { status: "verified" as const, source_url: "https://example.test" },
      docsUrl: null,
      directBaseUrl: "https://openrouter.ai/api",
    },
  ];
  const credentials = [
    {
      id: "openrouter-1",
      providerId: "openrouter",
      label: "primary",
      last4: "1f2a",
      priority: 0,
      enabled: true,
      legacy: false,
    },
  ];

  function renderPanel() {
    return render(
      <RoutingPanel
        config={{
          version: 1,
          enabled: true,
          targets: [
            {
              id: "openrouter-1",
              providerId: "openrouter",
              credentialId: "openrouter-1",
              model: "nvidia/nemotron-3-ultra",
              baseUrl: "https://openrouter.ai/api",
              enabled: true,
            },
          ],
        }}
        providers={providers}
        credentials={credentials}
        status={null}
        health={[
          {
            targetId: "openrouter-1",
            providerId: "openrouter",
            ok: false,
            message: "Key rejected (401).",
          },
        ]}
        onChange={() => {}}
        onSave={() => {}}
        onStartProxy={() => {}}
        onStopProxy={() => {}}
        onApplyRouted={() => {}}
      />,
    );
  }

  it("gives every control an accessible name", () => {
    renderPanel();
    for (const control of screen.getAllByRole("button")) {
      const name = control.getAttribute("aria-label") ?? control.textContent?.trim() ?? "";
      expect(name.length, `unnamed button: ${control.outerHTML.slice(0, 80)}`).toBeGreaterThan(0);
    }
    expect(screen.getByLabelText("Fail over between targets")).toBeInTheDocument();
    expect(screen.getByLabelText("Provider")).toBeInTheDocument();
    expect(screen.getByLabelText("Credential")).toBeInTheDocument();
  });

  it("never uses a positive tabindex and uses real disabled states", () => {
    renderPanel();
    for (const el of Array.from(document.querySelectorAll("[tabindex]"))) {
      expect(Number(el.getAttribute("tabindex"))).toBeLessThanOrEqual(0);
    }
    // "Route Claude Code" is enabled here; the first target cannot move up.
    expect(screen.getByRole("button", { name: "Move openrouter-1 up" })).toBeDisabled();
  });

  it("publishes failure reasons as text, not a hover-only title", () => {
    renderPanel();
    expect(screen.getByRole("list", { name: "Failing routing targets" })).toHaveTextContent(
      "Key rejected (401).",
    );
    expect(document.querySelector("[title]")).toBeNull();
  });
});
