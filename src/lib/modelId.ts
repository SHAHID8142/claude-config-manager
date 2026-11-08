// Model-id validation (T43) — TS mirror of Rust
// `config_merge::validate_model_id`, which is the apply-time authority.
// Keep both sides in sync; divergence is the kind of drift T19 guards against
// for endpoint classification.
//
// Rules (identical to Rust):
//   trim → non-empty, ≤ 256 chars, printable ASCII only (0x21..=0x7E),
//   i.e. no spaces, tabs, newlines or non-ASCII.
// Nothing else is constrained: provider id syntax varies too widely to
// guess at, and inventing extra rules would reject valid ids (PRD F03/F05).

export type ModelIdCheck = { ok: true; value: string } | { ok: false; message: string };

export const MODEL_ID_MAX_LEN = 256;

export function validateModelId(raw: string): ModelIdCheck {
  const value = raw.trim();
  if (value === "") {
    return { ok: false, message: "Enter a model ID, or select one from the catalog." };
  }
  if (value.length > MODEL_ID_MAX_LEN) {
    return { ok: false, message: `Model ID is longer than ${MODEL_ID_MAX_LEN} characters.` };
  }
  for (const ch of value) {
    const code = ch.codePointAt(0) ?? 0;
    if (code <= 0x20 || code >= 0x7f) {
      return {
        ok: false,
        message:
          "Model ID may only contain printable ASCII characters — no spaces or control characters.",
      };
    }
  }
  return { ok: true, value };
}
