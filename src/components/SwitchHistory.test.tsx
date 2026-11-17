import { render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import { createMockIpc } from "../lib/ipc-mock";
import { SwitchHistory } from "./SwitchHistory";
it("renders an out-of-range persisted timestamp without throwing", async () => {
  const ipc = createMockIpc({
    handlers: {
      switch_history: async () => [
        { epoch: Number.MAX_SAFE_INTEGER, providerId: "nvidia-nim", outcome: "auth" },
      ],
    },
  });
  render(<SwitchHistory ipc={ipc} />);
  expect(await screen.findByText("Unknown timestamp")).toBeInTheDocument();
  expect(screen.getByText(/Authentication or access rejected/)).toBeInTheDocument();
  expect(screen.getByText("Unknown timestamp")).not.toHaveAttribute("datetime");
});
it("keeps a machine-readable time for valid events", async () => {
  const ipc = createMockIpc({
    handlers: {
      switch_history: async () => [{ epoch: 1, providerId: "openrouter", outcome: "selected" }],
    },
  });
  const { container } = render(<SwitchHistory ipc={ipc} />);
  await screen.findByText(/Selected after fallback/);
  expect(container.querySelector("time")).toHaveAttribute("datetime", "1970-01-01T00:00:01.000Z");
});
