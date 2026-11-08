import { describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ApiKeyField } from "./ApiKeyField";

describe("ApiKeyField (T20)", () => {
  it("starts masked and never echoes the value as page text", () => {
    render(<ApiKeyField id="k" value="sk-test-DO-NOT-USE-0123456789" onChange={() => {}} />);
    const input = document.getElementById("k") as HTMLInputElement;
    expect(input.getAttribute("type")).toBe("password");
    // The raw value must not appear anywhere as visible text.
    expect(document.body.textContent).not.toContain("sk-test-DO-NOT-USE-0123456789");
  });

  it("reveals on demand and reports pressed state", async () => {
    const user = userEvent.setup();
    render(<ApiKeyField id="k" value="sk-test-abc" onChange={() => {}} />);
    const toggle = screen.getByRole("button", { name: "Show" });
    expect(toggle).toHaveAttribute("aria-pressed", "false");
    await user.click(toggle);
    const input = document.getElementById("k") as HTMLInputElement;
    expect(input.getAttribute("type")).toBe("text");
    expect(screen.getByRole("button", { name: "Hide" })).toHaveAttribute("aria-pressed", "true");
  });

  it("auto-rehides after the timeout (fake timers + fireEvent)", () => {
    vi.useFakeTimers();
    try {
      render(<ApiKeyField id="k" value="sk-test-abc" onChange={() => {}} />);
      const input = document.getElementById("k") as HTMLInputElement;
      fireEvent.click(screen.getByRole("button", { name: "Show" }));
      expect(input.getAttribute("type")).toBe("text");
      act(() => {
        vi.advanceTimersByTime(29_999);
      });
      expect(input.getAttribute("type")).toBe("text");
      act(() => {
        vi.advanceTimersByTime(1);
      });
      expect(input.getAttribute("type")).toBe("password");
    } finally {
      vi.useRealTimers();
    }
  });

  it("re-hides immediately on blur", async () => {
    const user = userEvent.setup();
    render(<ApiKeyField id="k" value="sk-test-abc" onChange={() => {}} />);
    const input = document.getElementById("k") as HTMLInputElement;
    await user.click(screen.getByRole("button", { name: "Show" }));
    expect(input.getAttribute("type")).toBe("text");
    await user.click(input); // focus
    fireEvent.blur(input);
    expect(input.getAttribute("type")).toBe("password");
  });

  it("propagates edits through onChange", async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<ApiKeyField id="k" value="" onChange={onChange} />);
    const input = document.getElementById("k") as HTMLInputElement;
    await user.type(input, "a");
    expect(onChange).toHaveBeenLastCalledWith("a");
  });

  it("keeps reveal toggle disabled with the field", () => {
    render(<ApiKeyField id="k" value="" onChange={() => {}} disabled />);
    expect(document.getElementById("k")).toBeDisabled();
    expect(screen.getByRole("button", { name: "Show" })).toBeDisabled();
  });

  it("forwards aria-describedby linkage from FieldRow", () => {
    render(<ApiKeyField id="k" value="" onChange={() => {}} aria-describedby="k-hint" />);
    expect(document.getElementById("k")).toHaveAttribute("aria-describedby", "k-hint");
  });
});
