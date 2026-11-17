/** Product explanations, separate from changing values and validation errors. */
export const FIELD_HELP: Record<string, string> = {
  Provider:
    "Choose the service that receives requests. Choosing here edits your setup; Apply activates it. NVIDIA uses the built-in local helper to translate requests. Other OpenAI-only providers need a supported translator.",
  "Custom provider name":
    "A label for your gateway. Its endpoint identifies its separate stored keys and model settings.",
  "Custom endpoint URL":
    "Use an Anthropic-compatible endpoint. The Messages URL appends /v1/messages; if the base already ends in /v1, the proxy appends only /messages.",
  "Endpoint URL":
    "This preset supplies the documented base URL. Use Custom to configure a different compatible gateway.",
  "API key":
    "A credential issued by this provider. Stored locally with owner-only permissions and passed through a helper; it is not copied into settings.json. Keys on one account may share limits.",
  Model:
    "The exact model ID accepted by this provider. Fetching the catalog never selects a model. Each fallback provider needs its own model mapping.",
  "Context window (tokens)":
    "An optional declared window for custom model IDs. Recognized Claude IDs and [1m] IDs follow different rules. Leave untouched to keep the saved value; use Reset to remove it. This does not expand a model's real capacity.",
  "Haiku role":
    "Optional model ID for the Haiku role. Leave blank to keep the saved override. Use this provider's model ID, not another provider's ID.",
  "Sonnet role":
    "Optional model ID for the Sonnet role on this provider. A blank field keeps its saved value.",
  "Opus role":
    "Optional model ID for the Opus role on this provider. A blank field keeps its saved value.",
};
