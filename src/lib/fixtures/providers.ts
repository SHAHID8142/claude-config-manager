// UI-side mirror of the Rust builtin presets (T17). This is a test/dev
// fixture copied from src-tauri/presets/anthropic.json — the Rust catalog is
// the source of truth; a drift check happens in T18's contract wiring.

import type { ProviderSummary } from "../ipc";

export interface ProviderTemplateFixture {
  schema_version: number;
  id: string;
  name: string;
  auth: "api_key" | "bearer_token";
  protocol: "anthropic_native" | "anthropic_compat" | "open_ai_only";
  bridge: "not_required" | "required_for_open_ai_only";
  direct_base_url: string | null;
  discovery:
    | { kind: "none" }
    | {
        kind: "endpoint";
        url: string;
        requires_auth: boolean;
        list_selector: string;
      };
  capability_status: { status: "verified"; source_url: string } | { status: "unknown" };
  docs_url: string | null;
}

export const anthropicPreset: ProviderTemplateFixture = {
  schema_version: 1,
  id: "anthropic",
  name: "Anthropic",
  auth: "api_key",
  protocol: "anthropic_native",
  bridge: "not_required",
  direct_base_url: null,
  discovery: {
    kind: "endpoint",
    url: "https://api.anthropic.com/v1/models",
    requires_auth: true,
    list_selector: "data",
  },
  capability_status: {
    status: "verified",
    source_url: "https://platform.claude.com/docs/en/api/models-list",
  },
  docs_url: "https://platform.claude.com/docs/en/api/models-list",
};

/** T24: OpenRouter preset — mirrors src-tauri/presets/openrouter.json. */
export const openRouterPreset: ProviderTemplateFixture = {
  schema_version: 1,
  id: "openrouter",
  name: "OpenRouter",
  auth: "bearer_token",
  protocol: "anthropic_compat",
  bridge: "not_required",
  direct_base_url: "https://openrouter.ai/api",
  discovery: {
    kind: "endpoint",
    url: "https://openrouter.ai/api/v1/models",
    requires_auth: false,
    list_selector: "data",
  },
  capability_status: {
    status: "verified",
    source_url: "https://openrouter.ai/docs/api-reference/messages",
  },
  docs_url: "https://openrouter.ai/docs/cookbook/coding-agents/claude-code-integration",
};

export const builtinPresetFixtures: ProviderTemplateFixture[] = [anthropicPreset, openRouterPreset];

/** T18: the provider list the UI receives via `list_providers`, mirrored
 * from the Rust `ProviderSummary` wire shape (camelCase). */
export const providerSummaryFixtures: ProviderSummary[] = [
  {
    id: "anthropic",
    name: "Anthropic",
    protocol: "anthropic_native",
    bridge: "not_required",
    capabilityStatus: {
      status: "verified",
      source_url: "https://platform.claude.com/docs/en/api/models-list",
    },
    docsUrl: "https://platform.claude.com/docs/en/api/models-list",
  },
  {
    id: "openrouter",
    name: "OpenRouter",
    protocol: "anthropic_compat",
    bridge: "not_required",
    capabilityStatus: {
      status: "verified",
      source_url: "https://openrouter.ai/docs/api-reference/messages",
    },
    docsUrl: "https://openrouter.ai/docs/cookbook/coding-agents/claude-code-integration",
  },
];
