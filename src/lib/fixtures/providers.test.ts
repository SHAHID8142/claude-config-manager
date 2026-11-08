import { describe, expect, it } from "vitest";
import { anthropicPreset, builtinPresetFixtures } from "./providers";

describe("builtin preset fixtures (T17)", () => {
  it("contains exactly the anthropic preset", () => {
    expect(builtinPresetFixtures).toHaveLength(1);
    expect(builtinPresetFixtures[0].id).toBe("anthropic");
  });

  it("anthropic preset is native protocol, direct mode, api_key auth", () => {
    expect(anthropicPreset.protocol).toBe("anthropic_native");
    expect(anthropicPreset.bridge).toBe("not_required");
    expect(anthropicPreset.direct_base_url).toBeNull();
    expect(anthropicPreset.auth).toBe("api_key");
  });

  it("discovery points at the documented models endpoint", () => {
    expect(anthropicPreset.discovery).toEqual({
      kind: "endpoint",
      url: "https://api.anthropic.com/v1/models",
      requires_auth: true,
      list_selector: "data",
    });
  });

  it("capability claims carry their verification source", () => {
    expect(anthropicPreset.capability_status).toEqual({
      status: "verified",
      source_url: "https://platform.claude.com/docs/en/api/models-list",
    });
  });
});
