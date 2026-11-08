import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AdvancedSettings } from "./AdvancedSettings";
import { EMPTY_ROLES, hasInvalidRole, roleChecks, rolesToSend } from "../lib/roles";

const baseProps = {
  open: true,
  onToggle: () => {},
  roles: EMPTY_ROLES,
  onChange: () => {},
  current: {},
};

describe("AdvancedSettings (T45)", () => {
  it("is collapsed by default and exposes its state via aria-expanded", () => {
    render(<AdvancedSettings {...baseProps} open={false} />);
    const toggle = screen.getByRole("button", { name: /advanced settings/i });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(toggle).toHaveAttribute("aria-controls", "advanced-panel");
    expect(screen.queryByRole("group", { name: /advanced model role overrides/i })).toBeNull();
    expect(screen.queryByLabelText("Haiku role")).toBeNull();
  });

  it("reveals three optional role fields when expanded", () => {
    render(<AdvancedSettings {...baseProps} />);
    expect(screen.getByRole("button", { name: /advanced settings/i })).toHaveAttribute(
      "aria-expanded",
      "true",
    );
    for (const label of ["Haiku role", "Sonnet role", "Opus role"]) {
      const input = screen.getByLabelText(label);
      expect(input).toHaveValue("");
      // Default-no-override is visible, not implied.
      expect(input).toHaveAttribute("placeholder", expect.stringMatching(/no override/i));
    }
  });

  it("calls onToggle when the disclosure button is pressed", async () => {
    const onToggle = vi.fn();
    render(<AdvancedSettings {...baseProps} open={false} onToggle={onToggle} />);
    await userEvent.click(screen.getByRole("button", { name: /advanced settings/i }));
    expect(onToggle).toHaveBeenCalledTimes(1);
  });

  it("reports a typed value to onChange and never preselects a model", async () => {
    const onChange = vi.fn();
    render(<AdvancedSettings {...baseProps} onChange={onChange} />);
    await userEvent.type(screen.getByLabelText("Sonnet role"), "x");
    expect(onChange).toHaveBeenLastCalledWith("sonnet", expect.stringContaining("x"));
  });

  it("shows the current effective value instead of implying nothing is set", () => {
    render(<AdvancedSettings {...baseProps} current={{ opus: "custom-opus" }} />);
    expect(screen.getByText("custom-opus")).toBeInTheDocument();
    expect(screen.getAllByText(/no override \(Claude Code default\)/).length).toBe(2);
  });

  it("flags invalid input accessibly and marks the field invalid", () => {
    render(<AdvancedSettings {...baseProps} roles={{ ...EMPTY_ROLES, haiku: "bad model" }} />);
    const input = screen.getByLabelText("Haiku role");
    expect(input).toHaveAttribute("aria-invalid", "true");
    // FieldRow appends its hint id; the error id must be present too.
    expect(input.getAttribute("aria-describedby")).toContain("role-haiku-error");
    expect(screen.getByText(/no spaces or control characters/i)).toBeInTheDocument();
  });

  it("blank fields produce no overrides at all (default-no-override)", () => {
    expect(rolesToSend(EMPTY_ROLES)).toEqual({});
    expect(rolesToSend({ haiku: "  ", sonnet: "", opus: "\t" })).toEqual({});
    expect(hasInvalidRole(EMPTY_ROLES)).toBe(false);
  });

  it("only validated, trimmed overrides are sent", () => {
    expect(rolesToSend({ haiku: "  fast-1  ", sonnet: "bad model", opus: "opus-9" })).toEqual({
      haiku: "fast-1",
      opus: "opus-9",
    });
    expect(hasInvalidRole({ haiku: "", sonnet: "bad model", opus: "" })).toBe(true);
    expect(roleChecks({ haiku: "", sonnet: "", opus: "" })).toEqual([
      { key: "haiku", kind: "none" },
      { key: "sonnet", kind: "none" },
      { key: "opus", kind: "none" },
    ]);
  });
});
