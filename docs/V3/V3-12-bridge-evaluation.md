# V3-12 — Evaluating an existing open-source bridge

**Outcome: no bridge was installed. This task is blocked on an owner decision and
on network access that was unavailable during this run.**

## What is actually required

An adapter that turns an Anthropic Messages request into OpenAI Chat Completions
and back must handle, at minimum:

| Anthropic (client) | OpenAI (provider) |
| --- | --- |
| `system` + `messages[].content` blocks | `messages[].role/content` |
| `tools[].input_schema` | `tools[].function.parameters` |
| `tool_use` / `tool_result` blocks | `tool_calls` / `role: "tool"` |
| `image` blocks | `image_url` parts |
| `max_tokens`, `temperature`, `top_p`, `stop_sequences` | same names / `stop` |
| streaming `content_block_delta` events | `chat.completion.chunk` deltas |
| `usage.input_tokens/output_tokens` | `usage.prompt_tokens/completion_tokens` |
| `stop_reason: end_turn/tool_use/max_tokens` | `finish_reason: stop/tool_calls/length` |
| `thinking` blocks | provider-specific (no portable mapping) |

A partial translation is worse than none: it silently loses tools, images or
stop reasons — which V3-16 explicitly forbids.

## Candidates considered

The repository already decided against configuring an external router
(ADR-003), and the V2 gap-closure left the question open. The candidate the
owner raised is **Claude Code Router** (the commonly used proxy that translates
Anthropic requests for OpenAI-style providers), plus writing our own translator
inside the helper.

| Candidate | What it would cost | What is unknown |
| --- | --- | --- |
| Claude Code Router as a bundled sidecar | needs its own Node runtime bundled, signed and notarised; a second health surface; a second config/credential store | licence and current maintenance status could **not** be verified in this session — the web search tool failed twice, and the repo contains no vendored copy to inspect |
| Our own adapter inside the helper | ~1.5–2k lines of translation + fixtures per provider; every new capability (vision, reasoning, tool streaming) becomes ours to keep correct | its correctness can only be proven against real provider responses, which need the owner's keys |
| A user-installed bridge on a port we detect | no code, no licence question, no runtime to ship | the user runs two tools; the app must not write the bridge's database (ADR-003) |
| Status quo (explicit refusal) | zero risk, honest error | OpenAI-only providers stay unusable inside the app |

## Recommendation

Keep the explicit refusal that V3-13/V3-16 now implement, and let the owner pick:

1. **User-installed bridge, detected.** The app recognises a bridge listening on
   loopback and can point a target at it; the app never writes its config and
   never claims to have installed it. Smallest change, no new runtime, no
   licence question.
2. **Bundle a verified translator.** Requires: a pinned version, licence and
   SHA-256 recorded in this repository, a network-free license review, and the
   owner accepting a second runtime inside the signed bundle.
3. **Write our own adapter.** Highest control, highest maintenance, and the only
   option where every capability decision is ours and testable.

**Why this is a hard stop rather than a guess:** the V3 brief forbids installing
an external binary/bridge and forbids describing an unimplemented capability as
supported. Choosing between these changes the architecture (a second runtime or a
new translation layer), which the rules require the owner to approve.

## What is already true today

- OpenAI-only providers are refused with `501 compatibility_error` and a message
  naming the missing bridge — never a silent 400 from the provider.
- Anthropic-compatible providers work end-to-end through the proxy without
  translation, because their endpoints accept the Anthropic wire format.
- No capability is silently dropped anywhere: what is not supported is refused.
