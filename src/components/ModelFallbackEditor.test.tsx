import { useState } from "react";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { ModelFallbackEditor } from "./ModelFallbackEditor";
import { createMockIpc } from "../lib/ipc-mock";
import type { IpcLike, RouteTarget } from "../lib/ipc";
const target: RouteTarget = {
  id: "nv",
  providerId: "nvidia-nim",
  credentialId: null,
  model: "primary-model",
  baseUrl: "https://integrate.api.nvidia.com",
  enabled: true,
};
function setup(initial = target, ipc: IpcLike = createMockIpc(), busy = false) {
  function Harness() {
    const [value, change] = useState(initial);
    return (
      <>
        <ModelFallbackEditor target={value} ipc={ipc} busy={busy} onChange={change} />
        <output data-testid="policy">{JSON.stringify(value)}</output>
      </>
    );
  }
  return render(<Harness />);
}
function policy(): RouteTarget {
  return JSON.parse(screen.getByTestId("policy").textContent ?? "{}");
}
describe("provider model fallback editor", () => {
  it("adds a manual fallback, changes primary by reorder, and removes a choice", async () => {
    setup();
    const user = userEvent.setup();
    await user.type(screen.getByLabelText("Add fallback model"), "second-model");
    await user.click(screen.getByRole("button", { name: "Add model" }));
    expect(policy().fallbackModels).toEqual(["second-model"]);
    await user.click(screen.getByRole("button", { name: "Move model 2 up" }));
    expect(policy().model).toBe("second-model");
    expect(policy().fallbackModels).toEqual(["primary-model"]);
    await user.click(screen.getByRole("button", { name: "Remove model 2" }));
    expect(policy().fallbackModels).toEqual([]);
    expect(screen.getByRole("button", { name: "Remove model 1" })).toBeDisabled();
  });
  it("rejects duplicate and invalid choices without clearing entered text", async () => {
    setup();
    const user = userEvent.setup();
    const field = screen.getByLabelText("Add fallback model");
    await user.type(field, "primary-model");
    await user.click(screen.getByRole("button", { name: "Add model" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("already in this provider");
    expect(field).toHaveValue("primary-model");
    await user.clear(field);
    await user.type(field, "contains space");
    await user.click(screen.getByRole("button", { name: "Add model" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("no spaces");
    expect(policy().fallbackModels).toBeUndefined();
  });
  it("loads suggestions explicitly and retains manual model IDs on catalog failure", async () => {
    const ipc = createMockIpc({
      handlers: {
        discover_models: async () => [{ id: "catalog-model", displayName: "Catalog model" }],
      },
    });
    setup(target, ipc);
    const user = userEvent.setup();
    expect(screen.queryByText(/suggestions loaded/)).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Load provider models" }));
    expect(await screen.findByText(/1 suggestions loaded/)).toBeInTheDocument();
    expect(document.querySelector('option[value="catalog-model"]')).toBeInTheDocument();
    await user.type(screen.getByLabelText("Add fallback model"), "manual-model");
    await user.click(screen.getByRole("button", { name: "Add model" }));
    expect(policy().fallbackModels).toEqual(["manual-model"]);
  });
  it("enforces the cap and disables editing while a save is in progress", async () => {
    const initial = { ...target, fallbackModels: ["two", "three", "four", "five"] };
    const user = userEvent.setup();
    const view = setup(initial);
    await user.type(screen.getByLabelText("Add fallback model"), "six");
    await user.click(screen.getByRole("button", { name: "Add model" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("at most 5");
    expect(policy().fallbackModels).toHaveLength(4);
    view.unmount();
    setup(initial, createMockIpc(), true);
    expect(screen.getByLabelText("Primary model")).toBeDisabled();
    expect(screen.getByRole("button", { name: "Add model" })).toBeDisabled();
  });
  it("shows a catalog error and still allows manual additions", async () => {
    setup(
      target,
      createMockIpc({
        handlers: {
          discover_models: async () => {
            throw new Error("fixture unavailable");
          },
        },
      }),
    );
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "Load provider models" }));
    await waitFor(() => expect(screen.getByRole("alert")).toHaveTextContent("manually"));
    await user.type(screen.getByLabelText("Add fallback model"), "manual");
    await user.click(screen.getByRole("button", { name: "Add model" }));
    expect(policy().fallbackModels).toEqual(["manual"]);
  });
});
