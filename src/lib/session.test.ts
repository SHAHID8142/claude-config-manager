import { describe, expect, it, beforeEach } from "vitest";
import { clearSessionString, loadSessionString, saveSessionString } from "./session";

describe("session persistence (D1)", () => {
  beforeEach(() => {
    clearSessionString("provider");
    clearSessionString("tab");
  });

  it("round-trips a value", () => {
    saveSessionString("provider", "anthropic");
    expect(loadSessionString("provider")).toBe("anthropic");
  });

  it("returns null for unset keys", () => {
    expect(loadSessionString("never-saved")).toBeNull();
  });

  it("clear removes the value", () => {
    saveSessionString("tab", "model");
    clearSessionString("tab");
    expect(loadSessionString("tab")).toBeNull();
  });
});
