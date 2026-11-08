import { describe, expect, it } from "vitest";
import { anthropicPreset, builtinPresetFixtures, openRouterPreset } from "./providers";

describe("builtin preset fixtures (T17)", () => {
  it("contains anthropic first, then verified presets in order", () => {
    expect(builtinPresetFixtures[0].id).toBe("anthropic");
    expect(builtinPresetFixtures[1].id).toBe("openrouter");
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

describe("openrouter preset fixture (T24)", () => {
  it("is direct anthropic_compat with bearer auth", () => {
    expect(openRouterPreset.protocol).toBe("anthropic_compat");
    expect(openRouterPreset.bridge).toBe("not_required");
    expect(openRouterPreset.auth).toBe("bearer_token");
    expect(openRouterPreset.direct_base_url).toBe("https://openrouter.ai/api");
  });

  it("declares the public documented models endpoint", () => {
    expect(openRouterPreset.discovery).toEqual({
      kind: "endpoint",
      url: "https://openrouter.ai/api/v1/models",
      requires_auth: false,
      list_selector: "data",
    });
  });
});
