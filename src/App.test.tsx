import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import App from "./App";

describe("App scaffold", () => {
  it("renders the Tauri template header", () => {
    render(<App />);
    expect(screen.getByRole("heading")).toBeInTheDocument();
  });
});
