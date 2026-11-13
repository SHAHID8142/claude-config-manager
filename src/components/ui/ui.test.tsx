import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Button } from "./Button";
import { SelectField } from "./SelectField";
import { Switch } from "./Switch";
import { TextInput } from "./TextInput";

describe("Button", () => {
  it("renders variants, sizes and never submits a form by default", () => {
    const { rerender } = render(<Button variant="primary">Save</Button>);
    const btn = screen.getByRole("button", { name: "Save" });
    expect(btn).toHaveAttribute("type", "button");
    expect(btn.className).toContain("btn-primary");
    rerender(
      <Button variant="danger" size="sm">
        Delete
      </Button>,
    );
    expect(screen.getByRole("button", { name: "Delete" }).className).toContain("btn-danger");
  });

  it("keeps icon-only decoration out of the accessibility tree", () => {
    render(<Button icon={<span data-testid="ico">x</span>}>Reload</Button>);
    expect(screen.getByRole("button", { name: "Reload" })).toBeInTheDocument();
    expect(screen.getByTestId("ico").parentElement).toHaveAttribute("aria-hidden", "true");
  });
});

describe("TextInput", () => {
  it("links the label, hint and error", () => {
    const { rerender } = render(<TextInput label="Endpoint" hint="HTTPS only" />);
    const input = screen.getByLabelText("Endpoint");
    expect(input).toHaveAccessibleDescription("HTTPS only");

    rerender(<TextInput label="Endpoint" hint="HTTPS only" error="Not a URL" />);
    expect(input).toHaveAttribute("aria-invalid", "true");
    expect(input).toHaveAccessibleDescription("Not a URL");
    expect(screen.getByRole("alert")).toHaveTextContent("Not a URL");
  });

  it("takes typed text", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<TextInput label="Model" onChange={onChange} />);
    await user.type(screen.getByLabelText("Model"), "ab");
    expect(onChange).toHaveBeenCalledTimes(2);
  });
});

describe("SelectField", () => {
  it("renders a placeholder and options and reports changes", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(
      <SelectField
        label="Provider"
        placeholder="Select a provider"
        value=""
        onChange={onChange}
        options={[{ value: "openrouter", label: "OpenRouter" }]}
      />,
    );
    const select = screen.getByLabelText("Provider");
    expect(select).toHaveDisplayValue("Select a provider");
    await user.selectOptions(select, "openrouter");
    expect(onChange).toHaveBeenCalled();
  });
});

describe("Switch", () => {
  it("toggles with the keyboard and reports state to assistive tech", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(
      <Switch label="Auto failover" checked={false} onChange={onChange} hint="Uses fallbacks" />,
    );
    const sw = screen.getByRole("switch", { name: "Auto failover" });
    expect(sw).toHaveAttribute("aria-checked", "false");
    sw.focus();
    await user.keyboard(" ");
    expect(onChange).toHaveBeenCalledWith(true);
    expect(sw).toHaveAccessibleDescription("Uses fallbacks");
  });
});
