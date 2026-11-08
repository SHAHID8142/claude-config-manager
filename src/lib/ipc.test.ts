import { describe, expect, it } from "vitest";
import { greet, isCommandError } from "./ipc";
import { createMockIpc } from "./ipc-mock";

describe("greet contract", () => {
  it("returns the greeting through the transport", async () => {
    const mock = createMockIpc({
      handlers: {
        greet: (args) => Promise.resolve(`Hello, ${args?.name}! You've been greeted from Rust!`),
      },
    });
    await expect(greet("fixture-user", mock)).resolves.toContain("fixture-user");
    expect(mock.calls).toEqual([{ cmd: "greet", args: { name: "fixture-user" } }]);
  });

  it("rejects oversized names client-side with a CommandError", async () => {
    const mock = createMockIpc();
    const long = "a".repeat(257);
    await expect(greet(long, mock)).rejects.toMatchObject({
      kind: "invalid_input",
      message: "name exceeds 256 characters",
    });
    // Must reject before touching the transport.
    expect(mock.calls).toHaveLength(0);
  });

  it("mirrors a Rust typed error on transport failure", async () => {
    const mock = createMockIpc({
      failures: { greet: { kind: "not_available", message: "backend offline" } },
    });
    await expect(greet("x", mock)).rejects.toMatchObject({
      kind: "not_available",
    });
  });
});

describe("isCommandError guard", () => {
  it("accepts a well-formed CommandError", () => {
    expect(isCommandError({ kind: "io", message: "read failed" })).toBe(true);
  });

  it("rejects non-conforming values", () => {
    expect(isCommandError("plain string")).toBe(false);
    expect(isCommandError(null)).toBe(false);
    expect(isCommandError({ kind: "unknown_kind", message: "x" })).toBe(false);
    expect(isCommandError({ kind: "io" })).toBe(false);
  });
});

describe("createMockIpc", () => {
  it("rejects unknown commands with not_available", async () => {
    const mock = createMockIpc();
    await expect(mock.invoke("missing_cmd")).rejects.toMatchObject({
      kind: "not_available",
      message: "no mock handler for command 'missing_cmd'",
    });
  });

  it("records invocation order for interaction tests", async () => {
    const mock = createMockIpc({
      handlers: { a: () => Promise.resolve(1), b: () => Promise.resolve(2) },
    });
    await mock.invoke("a");
    await mock.invoke("b", { k: 1 });
    expect(mock.calls.map((c) => c.cmd)).toEqual(["a", "b"]);
    expect(mock.calls[1].args).toEqual({ k: 1 });
  });
});
