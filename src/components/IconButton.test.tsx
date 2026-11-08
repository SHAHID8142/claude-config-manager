import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { RefreshCw, Trash2 } from "lucide-react";
import { IconButton } from "./IconButton";

describe("IconButton", () => {
  it("exposes its label as the accessible name and tooltip", () => {
    render(<IconButton label="Refresh models" icon={<RefreshCw />} />);
    const button = screen.getByRole("button", { name: "Refresh models" });
    expect(button).toHaveAttribute("title", "Refresh models");
    expect(button).toHaveAttribute("type", "button");
  });

  it("activates with keyboard and pointer", async () => {
    const user = userEvent.setup();
    const onClick = vi.fn();
    render(<IconButton label="Refresh" icon={<RefreshCw />} onClick={onClick} />);
    const button = screen.getByRole("button", { name: "Refresh" });
    button.focus();
    await user.keyboard("{Enter}");
    await user.keyboard(" ");
    await user.click(button);
    expect(onClick).toHaveBeenCalledTimes(3);
  });

  it("cannot be activated while disabled", async () => {
    const user = userEvent.setup();
    const onClick = vi.fn();
    render(<IconButton label="Delete" icon={<Trash2 />} disabled onClick={onClick} />);
    const button = screen.getByRole("button", { name: "Delete" });
    await user.click(button);
    expect(onClick).not.toHaveBeenCalled();
    expect(button).toBeDisabled();
  });

  it("applies the danger tone and requested size", () => {
    render(<IconButton label="Delete" icon={<Trash2 />} tone="danger" size="sm" />);
    const button = screen.getByRole("button", { name: "Delete" });
    expect(button.className).toContain("hover:text-danger");
    expect(button.className).toContain("size-7");
  });
});
