# V3-18 … V3-23 — Failover verification

Everything below is what the repository actually proves today. It was produced by
reading `src-tauri/src/failover.rs` and running
`tests/proxy_loopback.rs` (real sockets, scripted upstream servers) and the
unit tests in `failover.rs`.

## V3-18 — Failure classification

`failover::classify(status, body)` separates the cases that must behave
differently (unit test `classification_distinguishes_rate_limit_from_exhausted_quota`):

| Input | Class | Retried? |
| --- | --- | --- |
| 429 with a rate-limit body | `RateLimit` (temporary) | yes, after a short cooldown |
| 429 whose body says quota is exhausted | `Quota` (not temporary) | no — a 429 is not always temporary |
| 401 / 403 | `Auth` | no |
| 5xx | `Outage` | yes, with a longer cooldown |
| 400 / 422 | `Request` | no — the caller's payload is wrong |
| 404 on a model | `Model` | no |
| connect / timeout / TLS error | `Network` | yes |

`FailureClass::retryable()` is the only place that decides, so a new class cannot
accidentally start retrying.

## V3-19 — Retry and cooldown

- `parse_retry_after` accepts integer seconds only, ignores HTTP-date form, and
  clamps to a maximum, so a hostile or broken header cannot park a target for a
  week (`retry_after_is_bounded_and_seconds_only`).
- `cooldown_for(class, retry_after)` prefers `Retry-After`, else a per-class
  default; `Cooldowns` is in-memory, pruned, and cleared by the first success
  (`cooldowns_are_bounded_and_cleared_by_success`).
- At most one attempt per target per request; a request never loops.

## V3-20 — Credential selection

V3-20 is the task that closed a real hole (see `V3-02-defects.md` §6):
`proxy::resolve_secret` now requires the credential's provider to equal the
target's provider and requires the credential to be enabled.
Tests: `cross_provider_credential_is_never_resolved`,
`disabled_credential_is_never_routed_to`.

No rotation is performed behind the owner's back, and no key is used on a
provider it was not stored for. "Rotation" here means: an ordered chain where the
next target uses its own credential — never reusing one key to evade a provider's
quota or terms.

## V3-21 — Cross-provider fallback

`candidates()` returns enabled targets in configured order, skipping cooling ones
but never returning an empty list (so one target can always be tried).
`falls_back_on_rate_limit_and_reports_it` proves the second target answers after
the first returns 429. `routing_config` is not rewritten by the proxy: the
owner's priority order is read-only at request time.

## V3-22 — Streaming safety

- The request body is read fully before forwarding, so a request is only ever
  sent upstream once, whole.
- `stream_the_upstream_body_through` shows SSE bytes reaching the caller as they
  arrive (chunked, flushed per chunk).
- No code path re-sends a request after any byte of a response has been handed to
  the client: fallback only happens when the *upstream attempt* failed before a
  successful response existed. A begun stream is finished or broken, never
  replayed, so a duplicated tool call or double charge cannot come from here.
- `does_not_fail_over_on_a_request_error` proves a 400 from the first target is
  returned to Claude Code instead of being retried on the next provider.

## V3-23 — Recovery

`candidates_skip_cooling_targets_but_never_return_empty` plus
`cooldowns_are_bounded_and_cleared_by_success` cover: all targets cooling (the
chain still tries one), a temporary outage that recovers (cooldown expires and
the target is used again), and repeated 429s (each one re-marks the cooldown).
`reports_all_targets_failed_without_leaking_key_material` proves the failure
summary names targets and classes only — never a key, never a response body.

Restart behaviour is covered in `helper_lifecycle.rs`: cooldowns are memory-only
by design, so a restart clears them — stated explicitly rather than pretending
otherwise.

## Not verified

- A live 429 from a paid provider (would spend the owner's money).
- Real provider quota-exhaustion bodies beyond the documented shape.
- Behaviour when the upstream stalls *after* headers but before the first delta:
  the client read timeout closes the connection; the stream is not retried.
