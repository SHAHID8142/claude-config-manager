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
