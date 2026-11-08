import { describe, expect, it } from "vitest";
import {
  anthropicPreset,
  builtinPresetFixtures,
  deepSeekPreset,
  openRouterPreset,
} from "./providers";

describe("builtin preset fixtures (T17)", () => {
  it("contains anthropic first, then verified presets in order", () => {
    expect(builtinPresetFixtures[0].id).toBe("anthropic");
    expect(builtinPresetFixtures[1].id).toBe("openrouter");
    expect(builtinPresetFixtures[2].id).toBe("deepseek");
    expect(builtinPresetFixtures[3].id).toBe("zai");
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

describe("deepseek preset fixture (T24)", () => {
  it("is direct anthropic_compat with api_key auth", () => {
    expect(deepSeekPreset.protocol).toBe("anthropic_compat");
    expect(deepSeekPreset.bridge).toBe("not_required");
    expect(deepSeekPreset.auth).toBe("api_key");
    expect(deepSeekPreset.direct_base_url).toBe("https://api.deepseek.com/anthropic");
  });

  it("declares the authenticated documented models endpoint", () => {
    expect(deepSeekPreset.discovery).toEqual({
      kind: "endpoint",
      url: "https://api.deepseek.com/models",
      requires_auth: true,
      list_selector: "data",
    });
  });
});

describe("zai preset fixture (T24)", () => {
  it("is direct anthropic_compat with bearer auth and no discovery", () => {
    expect(zaiPreset.protocol).toBe("anthropic_compat");
    expect(zaiPreset.bridge).toBe("not_required");
    expect(zaiPreset.auth).toBe("bearer_token");
    expect(zaiPreset.direct_base_url).toBe("https://api.z.ai/api/anthropic");
    // Honest absence: no verified catalog endpoint -> manual model entry.
    expect(zaiPreset.discovery).toEqual({ kind: "none" });
  });
});
