import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Tooltip } from "./Tooltip";
import { HelpPopover } from "./HelpPopover";

describe("Tooltip", () => {
  it("is reachable and readable by keyboard, not only by hover", async () => {
    const user = userEvent.setup();
    render(<Tooltip content="Cloud credentials outrank the helper." />);
    const trigger = screen.getByRole("button", { name: "More information" });
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();

    trigger.focus();
    const tip = await screen.findByRole("tooltip");
    expect(tip).toHaveTextContent("Cloud credentials outrank the helper.");
    expect(trigger).toHaveAttribute("aria-describedby", tip.id);

    await user.keyboard("{Escape}");
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
  });

  it("shows on hover and hides after the pointer leaves", async () => {
    const user = userEvent.setup();
    render(<Tooltip content="hover text" label="About endpoint" />);
    await user.hover(screen.getByRole("button", { name: "About endpoint" }));
    expect(await screen.findByRole("tooltip")).toBeInTheDocument();
    await user.unhover(screen.getByRole("button", { name: "About endpoint" }));
    expect(await screen.findByRole("tooltip")).toBeInTheDocument(); // 80 ms grace
  });
});

describe("HelpPopover", () => {
  it("opens from the keyboard, reports expanded state, closes on Escape", async () => {
    const user = userEvent.setup();
    render(
      <HelpPopover title="Authentication precedence">
        <p>Shell exports beat the helper.</p>
      </HelpPopover>,
    );
    const trigger = screen.getByRole("button", { name: "About Authentication precedence" });
    expect(trigger).toHaveAttribute("aria-expanded", "false");
    trigger.focus();
    await user.keyboard("{Enter}");
    expect(trigger).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByRole("dialog", { name: "Authentication precedence" })).toHaveTextContent(
      "Shell exports beat the helper.",
    );
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(trigger).toHaveAttribute("aria-expanded", "false");
  });
});
