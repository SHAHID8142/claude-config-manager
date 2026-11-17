import { describe, expect, it } from "vitest";
import { loadProviderDraft, saveProviderDraft } from "./providerDraft";
const draft = (n: number) => ({
  model: `model-${n}`,
  roles: { haiku: "h".repeat(256), sonnet: "s".repeat(256), opus: "o".repeat(256) },
  context: "1000000",
  resetContext: false,
  resetRoles: [],
  customName: `Gateway ${n}`,
  customEndpoint: `https://example.com/${"p".repeat(1800)}/${n}`,
});
describe("bounded durable provider drafts", () => {
  it("keeps recent valid entries readable when the collection exceeds its text cap", () => {
    for (let n = 0; n < 40; n++) saveProviderDraft(`custom-${n}`, draft(n));
    const raw = window.localStorage.getItem("ccm.session.provider-drafts-v1")!;
    expect(raw.length).toBeLessThanOrEqual(65536);
    expect(Object.keys(JSON.parse(raw)).length).toBeLessThanOrEqual(32);
    expect(loadProviderDraft("custom-39").model).toBe("model-39");
    for (const [id, value] of Object.entries(JSON.parse(raw)))
      expect(loadProviderDraft(id)).toEqual(value);
  });
  it("retains the most recently edited provider when evicting older entries", () => {
    for (let n = 0; n < 20; n++) saveProviderDraft(`custom-${n}`, draft(n));
    saveProviderDraft("custom-0", draft(100));
    for (let n = 20; n < 25; n++) saveProviderDraft(`custom-${n}`, draft(n));
    expect(loadProviderDraft("custom-0").model).toBe("model-100");
  });
});
