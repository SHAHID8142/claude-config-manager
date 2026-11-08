import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { StatusBanner, type FeedbackSeverity } from "./StatusBanner";

describe("StatusBanner", () => {
  it.each<[FeedbackSeverity, string, string]>([
    ["error", "alert", "assertive"],
    ["warning", "status", "polite"],
    ["success", "status", "polite"],
    ["info", "status", "polite"],
  ])("severity %s uses role=%s with aria-live=%s", (severity, role, live) => {
    render(<StatusBanner severity={severity} message="test message" />);
    const banner = screen.getByText("test message").parentElement?.parentElement;
    expect(banner).toHaveAttribute("role", role);
    expect(banner).toHaveAttribute("aria-live", live);
  });

  it("renders detail text when provided", () => {
    render(<StatusBanner severity="error" message="Apply failed" detail="Backup restored." />);
    expect(screen.getByText("Backup restored.")).toBeInTheDocument();
  });

  it("announces severity to screen readers via visually hidden label", () => {
    render(<StatusBanner severity="error" message="boom" />);
    expect(screen.getByText("Error:")).toHaveClass("sr-only");
  });

  it("dismisses via the dismiss control", async () => {
    const onDismiss = vi.fn();
    render(<StatusBanner severity="info" message="note" onDismiss={onDismiss} />);
    await userEvent.click(screen.getByRole("button", { name: "Dismiss message" }));
    expect(onDismiss).toHaveBeenCalledOnce();
  });

  it("renders without a dismiss control when not dismissible", () => {
    render(<StatusBanner severity="info" message="note" />);
    expect(screen.queryByRole("button", { name: "Dismiss message" })).not.toBeInTheDocument();
  });
});
