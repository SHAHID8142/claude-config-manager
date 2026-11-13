import { describe, expect, it } from "vitest";
import { isCommandError } from "./ipc";
import { createMockIpc } from "./ipc-mock";

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
