import { describe, expect, it } from "vitest";
import { MODEL_ID_MAX_LEN, validateModelId } from "./modelId";

describe("validateModelId (T43) — mirror of Rust validate_model_id", () => {
  it("accepts real provider model ids", () => {
    for (const id of [
      "claude-sonnet-4-5",
      "anthropic/claude-x",
      "deepseek/deepseek-y",
      "glm-4.6",
      "models/gemini-2.5-pro",
      "m",
    ]) {
      expect(validateModelId(id)).toEqual({ ok: true, value: id });
    }
  });

  it("trims surrounding whitespace and returns the normalized value", () => {
    expect(validateModelId("  claude-sonnet-4-5  ")).toEqual({
      ok: true,
      value: "claude-sonnet-4-5",
    });
  });

  it("rejects empty and whitespace-only input", () => {
    for (const bad of ["", "   ", "\t", "\n"]) {
      const result = validateModelId(bad);
      expect(result.ok).toBe(false);
    }
  });

  it("rejects embedded spaces and control characters", () => {
    for (const bad of ["bad model", "model\nid", "model\tid", "model\u0000id"]) {
      const result = validateModelId(bad);
      expect(result.ok).toBe(false);
      if (!result.ok) expect(result.message).toMatch(/printable ASCII/);
    }
  });

  it("rejects non-ASCII identifiers (mirrors is_ascii_graphic)", () => {
    for (const bad of ["modèle", "モデル", "model🦀"]) {
      expect(validateModelId(bad).ok).toBe(false);
    }
  });

  it("enforces the 256-character cap from both sides", () => {
    expect(validateModelId("a".repeat(MODEL_ID_MAX_LEN)).ok).toBe(true);
    const tooLong = validateModelId("a".repeat(MODEL_ID_MAX_LEN + 1));
    expect(tooLong.ok).toBe(false);
    if (!tooLong.ok) expect(tooLong.message).toMatch(/longer than 256/);
  });
});
