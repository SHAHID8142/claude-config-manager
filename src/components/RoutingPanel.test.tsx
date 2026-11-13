/** V2-23/24/30: the routing chain editor behaves like an ordered list. */
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { RoutingPanel } from "./RoutingPanel";
import type { CredentialView, ProviderSummary, RouteTarget, RoutingConfig } from "../lib/ipc";

const providers: ProviderSummary[] = [
  {
    id: "anthropic",
    name: "Anthropic",
    protocol: "anthropic_native",
    bridge: "not_required",
    capabilityStatus: { status: "verified", source_url: "https://example.test" },
    docsUrl: null,
    directBaseUrl: null,
  },
  {
    id: "openrouter",
    name: "OpenRouter",
    protocol: "anthropic_compat",
    bridge: "not_required",
    capabilityStatus: { status: "verified", source_url: "https://example.test" },
    docsUrl: "https://openrouter.ai/docs",
    directBaseUrl: "https://openrouter.ai/api",
  },
];

const credentials: CredentialView[] = [
  {
    id: "openrouter-1",
    providerId: "openrouter",
    label: "primary",
    last4: "1f2a",
    priority: 0,
    enabled: true,
    legacy: false,
  },
  {
    id: "openrouter-2",
    providerId: "openrouter",
    label: "backup",
    last4: "9e8d",
    priority: 1,
    enabled: false,
    legacy: false,
  },
];

function target(id: string, overrides: Partial<RouteTarget> = {}): RouteTarget {
  return {
    id,
    providerId: "openrouter",
    credentialId: "openrouter-1",
    model: "nvidia/nemotron-3-ultra",
    baseUrl: "https://openrouter.ai/api",
    enabled: true,
    ...overrides,
  };
}

function config(targets: RouteTarget[]): RoutingConfig {
  return { version: 1, enabled: true, targets };
}

function setup(overrides: Partial<React.ComponentProps<typeof RoutingPanel>> = {}) {
  const onChange = vi.fn();
  const onSave = vi.fn();
  const onStartProxy = vi.fn();
  const onStopProxy = vi.fn();
  const onApplyRouted = vi.fn();
  render(
    <RoutingPanel
      config={config([target("a"), target("b")])}
      providers={providers}
      credentials={credentials}
      status={null}
      onChange={onChange}
      onSave={onSave}
      onStartProxy={onStartProxy}
      onStopProxy={onStopProxy}
      onApplyRouted={onApplyRouted}
      {...overrides}
    />,
  );
  return { onChange, onSave, onStartProxy, onStopProxy, onApplyRouted };
}

describe("RoutingPanel", () => {
  it("lists targets in order and moves one up without losing it", async () => {
    const { onChange } = setup();
    const items = screen.getAllByRole("listitem");
    expect(items).toHaveLength(2);
    await userEvent.click(screen.getByRole("button", { name: "Move b up" }));
    const next = onChange.mock.calls[0][0] as RoutingConfig;
    expect(next.targets.map((t) => t.id)).toEqual(["b", "a"]);
  });

  it("cannot move the first target up or the last one down", () => {
    setup();
    expect(screen.getByRole("button", { name: "Move a up" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Move b down" })).toBeDisabled();
  });

  it("removes a target by id", async () => {
    const { onChange } = setup();
    await userEvent.click(screen.getByRole("button", { name: "Remove a" }));
    const next = onChange.mock.calls[0][0] as RoutingConfig;
    expect(next.targets.map((t) => t.id)).toEqual(["b"]);
  });

  it("toggles failover and saves the chain", async () => {
    const { onChange, onSave } = setup();
    await userEvent.click(screen.getByLabelText("Fail over between targets"));
    expect((onChange.mock.calls[0][0] as RoutingConfig).enabled).toBe(false);
    await userEvent.click(screen.getByRole("button", { name: "Save chain" }));
    expect(onSave).toHaveBeenCalledOnce();
  });

  it("adds a target only with a provider and an enabled credential", async () => {
    const { onChange } = setup({ config: config([]) });
    const add = screen.getByRole("button", { name: "Add target" });
    expect(add).toBeDisabled();

    await userEvent.selectOptions(screen.getByLabelText("Provider"), "openrouter");
    // Only enabled credentials are offered: the disabled "backup" is absent.
    const options = Array.from(screen.getByLabelText("Credential").querySelectorAll("option")).map(
      (o) => o.textContent,
    );
    expect(options).toEqual(["Choose…", "primary"]);

    await userEvent.selectOptions(screen.getByLabelText("Credential"), "openrouter-1");
    expect(add).toBeEnabled();
    await userEvent.click(add);
    const next = onChange.mock.calls[0][0] as RoutingConfig;
    expect(next.targets).toHaveLength(1);
    expect(next.targets[0].baseUrl).toBe("https://openrouter.ai/api");
    expect(next.targets[0].credentialId).toBe("openrouter-1");
  });

  it("reports per-target key health and lets the owner re-check", async () => {
    const onCheckHealth = vi.fn();
    setup({
      onCheckHealth,
      health: [
        { targetId: "a", providerId: "openrouter", ok: true, message: "Key accepted." },
        { targetId: "b", providerId: "openrouter", ok: false, message: "Key rejected (401)." },
      ],
    });
    expect(screen.getByText("key ok")).toBeInTheDocument();
    expect(screen.getByText("key failed")).toBeInTheDocument();
    // The reason is real text, not a hover-only title attribute.
    expect(screen.getByRole("list", { name: "Failing routing targets" })).toHaveTextContent(
      "b: Key rejected (401).",
    );
    await userEvent.click(screen.getByRole("button", { name: "Check targets" }));
    expect(onCheckHealth).toHaveBeenCalledOnce();
  });

  it("cannot route Claude Code with an empty chain", () => {
    setup({ config: config([]) });
    expect(screen.getByRole("button", { name: "Route Claude Code" })).toBeDisabled();
  });

  it("routes Claude Code once the chain has a target", async () => {
    const { onApplyRouted } = setup({ config: config([target("a")]) });
    const apply = screen.getByRole("button", { name: "Route Claude Code" });
    expect(apply).toBeEnabled();
    await userEvent.click(apply);
    expect(onApplyRouted).toHaveBeenCalledOnce();
  });

  it("shows the proxy state and marks a target that is cooling down", () => {
    setup({
      status: {
        running: true,
        port: 51234,
        requests: 12,
        fallbacks: 2,
        failures: 1,
        lastTarget: "b",
        lastError: null,
        cooling: ["a"],
      },
    });
    expect(screen.getByText("running on 127.0.0.1:51234")).toBeInTheDocument();
    expect(screen.getByText(/12 requests/)).toBeInTheDocument();
    expect(screen.getByText("cooling down")).toBeInTheDocument();
    // D6: one state-dependent proxy button + the primary route action.
    expect(screen.getByRole("button", { name: "Stop proxy" })).toBeEnabled();
    expect(screen.queryByRole("button", { name: "Start proxy" })).not.toBeInTheDocument();
  });
});
