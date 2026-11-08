import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { CredentialsPanel } from "./CredentialsPanel";
import { maskKey } from "../lib/maskKey";
import type { CredentialView } from "../lib/ipc";

const base: CredentialView = {
  id: "openrouter-1",
  providerId: "openrouter",
  label: "work",
  last4: "1f2a",
  priority: 0,
  enabled: true,
  legacy: false,
};

function creds(overrides: Partial<CredentialView>[]): CredentialView[] {
  return overrides.map((o, i) => ({ ...base, id: `c-${i}`, ...o }));
}

const noop = {
  onAdd: () => {},
  onRemove: () => {},
  onToggle: () => {},
  onPromote: () => {},
};

describe("CredentialsPanel (V2-21)", () => {
  it("masks every key and never renders the secret", () => {
    render(<CredentialsPanel {...noop} credentials={creds([{}, {}])} />);
    expect(screen.getAllByText("•••• 1f2a")).toHaveLength(2);
    expect(document.body.textContent).not.toMatch(/sk-|1f2a1f2a/);
    expect(maskKey("")).toBe("••••");
  });

  it("marks a migrated key as existing and offers no delete for it", () => {
    render(
      <CredentialsPanel
        {...noop}
        credentials={creds([
          { id: "openrouter-legacy", label: "Existing key", last4: "", legacy: true },
        ])}
      />,
    );
    expect(screen.getByText("existing")).toBeInTheDocument();
    expect(screen.getByText("••••")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /delete/i })).toBeNull();
  });

  it("disables, promotes and removes by explicit action only", async () => {
    const onToggle = vi.fn();
    const onPromote = vi.fn();
    const onRemove = vi.fn();
    render(
      <CredentialsPanel
        {...noop}
        onToggle={onToggle}
        onPromote={onPromote}
        onRemove={onRemove}
        credentials={creds([{ id: "openrouter-2", label: "free", priority: 1 }])}
      />,
    );
    await userEvent.click(screen.getByRole("switch", { name: "Enable free" }));
    expect(onToggle).toHaveBeenCalledWith("openrouter-2", false);
    await userEvent.click(screen.getByRole("button", { name: /try free first/i }));
    expect(onPromote).toHaveBeenCalledWith("openrouter-2");
    await userEvent.click(screen.getByRole("button", { name: /delete free/i }));
    expect(onRemove).toHaveBeenCalledWith("openrouter-2");
  });

  it("adds a key with a label and clears the fields afterwards", async () => {
    const onAdd = vi.fn();
    render(<CredentialsPanel {...noop} onAdd={onAdd} credentials={[]} />);
    await userEvent.type(screen.getByLabelText(/label for the new key/i), "second");
    await userEvent.type(screen.getByLabelText(/new api key/i), "sk-test-fixture");
    await userEvent.click(screen.getByRole("button", { name: "Add" }));
    expect(onAdd).toHaveBeenCalledWith("second", "sk-test-fixture");
    expect(screen.getByLabelText(/new api key/i)).toHaveValue("");
  });

  it("cannot add a blank key and keeps the button disabled", () => {
    render(<CredentialsPanel {...noop} credentials={[]} />);
    expect(screen.getByRole("button", { name: "Add" })).toBeDisabled();
    expect(screen.getByText(/never shown again/i)).toBeInTheDocument();
  });
});
