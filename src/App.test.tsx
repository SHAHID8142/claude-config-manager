import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import App from "./App";

describe("App shell (T12)", () => {
  it("renders the app title", () => {
    render(<App />);
    expect(screen.getByRole("heading", { name: "Claude Config Manager" })).toBeInTheDocument();
  });

  it("renders all PRD main-screen sections", () => {
    render(<App />);
    for (const section of ["Active configuration", "Provider", "Model", "Actions"]) {
      expect(screen.getByRole("region", { name: section })).toBeInTheDocument();
    }
  });

  it("marks controls not yet wired as disabled with honest placeholder status", () => {
    render(<App />);
    // Section titles and field labels can share words ("Provider"), so
    // disambiguate by control element type.
    expect(screen.getByLabelText("Provider", { selector: "select" })).toBeDisabled();
    expect(screen.getByLabelText("Endpoint URL", { selector: "input" })).toBeDisabled();
    expect(screen.getByLabelText("API key", { selector: "input" })).toBeDisabled();
    expect(screen.getByLabelText("Model", { selector: "input" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Test Connection" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Save & Apply" })).toBeDisabled();
    expect(screen.getByText("mode: not configured")).toBeInTheDocument();
  });

  it("provides the connection-mode indicator with an accessible name", () => {
    render(<App />);
    expect(screen.getByLabelText("Connection mode")).toBeInTheDocument();
  });
});

describe("Accessibility (T14)", () => {
  it("links field hints to their controls via aria-describedby", () => {
    render(<App />);
    const endpoint = screen.getByLabelText("Endpoint URL", { selector: "input" });
    expect(endpoint).toHaveAttribute("aria-describedby", "endpoint-input-hint");
    expect(screen.getByText("Editable when a provider is selected.")).toHaveAttribute(
      "id",
      "endpoint-input-hint",
    );
    const apiKey = screen.getByLabelText("API key", { selector: "input" });
    expect(apiKey).toHaveAttribute("aria-describedby", "api-key-input-hint");
  });

  it("links hints on every hinted field, not just the first", () => {
    render(<App />);
    const model = screen.getByLabelText("Model", { selector: "input" });
    expect(model).toHaveAttribute("aria-describedby", "model-input-hint");
    const provider = screen.getByLabelText("Provider", { selector: "select" });
    expect(provider).toHaveAttribute("aria-describedby", "provider-select-hint");
  });

  it("applies visible focus treatments to every interactive control", () => {
    render(<App />);
    const focusables = [
      screen.getByLabelText("Provider", { selector: "select" }),
      screen.getByLabelText("Endpoint URL", { selector: "input" }),
      screen.getByLabelText("API key", { selector: "input" }),
      screen.getByLabelText("Model", { selector: "input" }),
      screen.getByRole("button", { name: "Test Connection" }),
      screen.getByRole("button", { name: "Save & Apply" }),
      screen.getByText("Advanced"),
    ];
    for (const el of focusables) {
      const cls = el.getAttribute("class") ?? "";
      expect(
        cls.includes("focus-visible:ring-2") || el.tagName === "SUMMARY",
        `missing focus treatment on <${el.tagName}>`,
      ).toBe(true);
    }
  });
});
