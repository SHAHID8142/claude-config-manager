import { describe, expect, it } from "vitest";
import { classifyEndpoint, endpointCheckMessage } from "./endpoint";

describe("classifyEndpoint (T19 mirror of Rust)", () => {
  it("accepts https remote endpoints", () => {
    expect(classifyEndpoint("https://api.example.com/v1")).toEqual({ check: "remote_ok" });
  });

  it("refuses non-https schemes", () => {
    expect(classifyEndpoint("http://api.example.com")).toEqual({ check: "insecure" });
    expect(classifyEndpoint("ftp://x")).toEqual({ check: "insecure" });
  });

  it("classifies local endpoints as local", () => {
    expect(classifyEndpoint("https://localhost:8080")).toEqual({ check: "local" });
    expect(classifyEndpoint("https://127.0.0.1:31415")).toEqual({ check: "local" });
    expect(classifyEndpoint("https://mybox.local")).toEqual({ check: "local" });
  });

  it("rejects garbage URLs", () => {
    expect(classifyEndpoint("::: not a url").check).toBe("invalid");
    expect(classifyEndpoint("").check).toBe("invalid");
  });

  it("produces user-facing messages, null when OK", () => {
    expect(endpointCheckMessage({ check: "remote_ok" })).toBeNull();
    expect(endpointCheckMessage({ check: "insecure" })).toMatch(/insecure/i);
    expect(endpointCheckMessage({ check: "local" })).toMatch(/local/i);
    expect(endpointCheckMessage({ check: "invalid", detail: "x" })).toMatch(/not a valid URL/i);
  });
});
