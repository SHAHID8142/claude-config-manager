import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ModelList } from "./ModelList";

const CATALOG_SOURCE = "https://openrouter.ai/api/v1/models";

const models = [
  {
    id: "anthropic/claude-x",
    displayName: "Claude X",
    capabilityStatus: "unknown" as const,
    source: CATALOG_SOURCE,
  },
  {
    id: "deepseek/deepseek-y",
    displayName: null,
    capabilityStatus: "unknown" as const,
    source: CATALOG_SOURCE,
  },
  {
    id: "openai/gpt-z",
    displayName: "GPT Z",
    capabilityStatus: "unknown" as const,
    source: CATALOG_SOURCE,
  },
];

describe("ModelList (T40)", () => {
  it("renders all models with unknown-capability chips", () => {
    render(<ModelList models={models} selectedId={null} onSelect={() => {}} />);
    // The list container keeps its aria-label for AT; options are buttons.
    expect(screen.getByLabelText("Discovered models")).toBeInTheDocument();
    expect(screen.getAllByText("capabilities unknown")).toHaveLength(3);
    expect(screen.getByText(/3 of 3 models shown/)).toBeInTheDocument();
  });

  it("filters by id and display name, with honest empty state", async () => {
    render(<ModelList models={models} selectedId={null} onSelect={() => {}} />);
    const search = screen.getByLabelText("Filter models");
    await userEvent.type(search, "deepseek");
    expect(screen.getAllByRole("option")).toHaveLength(1);
    expect(screen.getByText(/1 of 3 models shown/)).toBeInTheDocument();

    await userEvent.clear(search);
    await userEvent.type(search, "zzz-nothing");
    expect(screen.getByText(/No models match/)).toBeInTheDocument();
    expect(screen.queryByRole("option")).not.toBeInTheDocument();
  });

  it("selects only on explicit click and never by default", async () => {
    const onSelect = vi.fn();
    const { rerender } = render(
      <ModelList models={models} selectedId={null} onSelect={onSelect} />,
    );
    // Nothing selected on initial render (CSS check avoids the RTL
    // aria-string gotcha with { selected: true }).
    expect(document.querySelectorAll("[aria-selected='true']")).toHaveLength(0);

    await userEvent.click(screen.getByRole("option", { name: /anthropic\/claude-x/ }));
    expect(onSelect).toHaveBeenCalledExactlyOnceWith("anthropic/claude-x");

    // With a selection, exactly one option is marked selected.
    rerender(<ModelList models={models} selectedId="anthropic/claude-x" onSelect={onSelect} />);
    const selected = document.querySelectorAll("[aria-selected='true']");
    expect(selected).toHaveLength(1);
    expect(selected[0]).toHaveAttribute("aria-selected", "true");
  });
});

describe("Model metadata provenance (T42)", () => {
  it("shows the verified catalog source and keeps capability data unknown", () => {
    render(<ModelList models={models} selectedId={null} onSelect={() => {}} />);
    expect(screen.getByText(CATALOG_SOURCE)).toBeInTheDocument();
    expect(screen.getByText(/capability data unverified/)).toBeInTheDocument();
    // No invented capability values anywhere in the rendered surface (F12).
    expect(document.body.textContent).not.toMatch(/context window|vision|reasoning|tokens/i);
  });

  it("labels a missing source as unknown instead of guessing", () => {
    render(
      <ModelList
        models={[{ id: "m", displayName: null, capabilityStatus: "unknown", source: "  " }]}
        selectedId={null}
        onSelect={() => {}}
      />,
    );
    expect(screen.getByText("unknown")).toBeInTheDocument();
    expect(screen.queryByText(CATALOG_SOURCE)).not.toBeInTheDocument();
  });
});
