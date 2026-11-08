import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AppSidebar } from "./AppSidebar";
import { TABS } from "../lib/tab";

describe("AppSidebar", () => {
  it("lists every screen and marks only the current one", () => {
    render(<AppSidebar active="credentials" onSelect={() => undefined} />);
    for (const tab of TABS) {
      expect(screen.getByRole("button", { name: tab.label })).toBeInTheDocument();
    }
    expect(screen.getByRole("button", { name: "Credentials" })).toHaveAttribute(
      "aria-current",
      "page",
    );
    expect(screen.getByRole("button", { name: "Dashboard" })).not.toHaveAttribute("aria-current");
  });

  it("switches screens on click and from the keyboard", async () => {
    const user = userEvent.setup();
    const onSelect = vi.fn();
    render(<AppSidebar active="dashboard" onSelect={onSelect} />);

    await user.click(screen.getByRole("button", { name: "Failover" }));
    expect(onSelect).toHaveBeenCalledWith("routing");

    // Buttons respond to Enter/Space natively — no custom key handling needed.
    screen.getByRole("button", { name: "Model" }).focus();
    await user.keyboard("{Enter}");
    expect(onSelect).toHaveBeenCalledWith("model");
  });

  it("keeps badge counts out of the accessible name", () => {
    render(
      <AppSidebar
        active="dashboard"
        onSelect={() => undefined}
        badges={{ credentials: "3", routing: "on" }}
      />,
    );
    // The count is decoration; a screen reader hears "Credentials", not "Credentials 3".
    expect(screen.getByRole("button", { name: "Credentials" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Credentials 3/ })).not.toBeInTheDocument();
    expect(screen.getByText("3")).toHaveAttribute("aria-hidden", "true");
  });
});
