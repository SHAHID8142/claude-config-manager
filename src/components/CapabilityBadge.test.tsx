import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { BridgeBadge, CapabilityBadge } from "./CapabilityBadge";

describe("CapabilityBadge (T25)", () => {
  it("renders verified with a source link", () => {
    render(<CapabilityBadge status="verified" sourceUrl="https://example.com/docs" />);
    expect(screen.getByText(/capabilities verified/)).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "(source)" })).toHaveAttribute(
      "href",
      "https://example.com/docs",
    );
  });

  it("renders verified without a link when no source exists", () => {
    render(<CapabilityBadge status="verified" sourceUrl={null} />);
    expect(screen.queryByRole("link")).not.toBeInTheDocument();
    expect(screen.getByText(/capabilities verified/)).toBeInTheDocument();
  });

  it("renders unknown as a first-class state (no fabricated claims)", () => {
    render(<CapabilityBadge status="unknown" />);
    expect(screen.getByText(/capabilities unknown/)).toBeInTheDocument();
    expect(screen.queryByRole("link", { name: "(source)" })).not.toBeInTheDocument();
  });
});

describe("BridgeBadge (T25)", () => {
  it("renders direct connection when not required", () => {
    render(<BridgeBadge required={false} />);
    expect(screen.getByText(/direct connection/)).toBeInTheDocument();
  });

  it("renders bridge-required with an honest availability note", () => {
    render(<BridgeBadge required />);
    expect(screen.getByText(/bridge required/)).toBeInTheDocument();
    expect(screen.getByText(/arrives in a later milestone/)).toBeInTheDocument();
  });
});
