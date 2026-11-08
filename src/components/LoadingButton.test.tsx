import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { LoadingButton } from "./LoadingButton";

describe("LoadingButton", () => {
  it("renders label and is clickable when idle", async () => {
    const onClick = vi.fn();
    render(<LoadingButton label="Save & Apply" onClick={onClick} />);
    const btn = screen.getByRole("button", { name: "Save & Apply" });
    expect(btn).toHaveAttribute("aria-busy", "false");
    await userEvent.click(btn);
    expect(onClick).toHaveBeenCalledOnce();
  });

  it("shows pending label, aria-busy, and is inert while pending", async () => {
    const onClick = vi.fn();
    render(
      <LoadingButton label="Save & Apply" pendingLabel="Applying…" pending onClick={onClick} />,
    );
    const btn = screen.getByRole("button", { name: "Applying…" });
    expect(btn).toBeDisabled();
    expect(btn).toHaveAttribute("aria-busy", "true");
    await userEvent.click(btn);
    expect(onClick).not.toHaveBeenCalled();
    // Spinner is present and hidden from assistive tech.
    expect(btn.querySelector("span[aria-hidden='true']")).not.toBeNull();
  });

  it("respects disabled prop independently of pending", () => {
    render(<LoadingButton label="Test Connection" disabled />);
    expect(screen.getByRole("button", { name: "Test Connection" })).toBeDisabled();
  });
});
