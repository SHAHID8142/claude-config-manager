# T19 — Validated editable Base URL field + secure protocol warnings

**Date:** 2026-11-08 · **Result:** ✅ PASS

## Implementation
- **Rust authority:** `provider_catalog::classify_endpoint` — tagged `EndpointCheck` (`invalid{detail}` / `insecure` / `local` / `remote_ok`). Non-https schemes refused (credentials would travel in cleartext — threat T11); localhost/127.0.0.1/::1/*.local classified `local` (legitimate for local gateways; disclosed as "traffic stays on this machine"); https remote = ok.
- **TS mirror:** `src/lib/endpoint.ts` with identical semantics + user-facing messages (`endpointCheckMessage`). Divergence guarded by mirror tests on both sides and the T67 full-regression review.
- **UI:** custom endpoint field classifies live while typing; error (invalid/insecure) renders red with `aria-live="polite"`; local renders an informational note; remote-https renders nothing (clean state). Field hint updated.
- Preset endpoints remain preset-controlled/read-only (validated https by the T16 schema gate).

## Security review
- http:// (and any non-https scheme) is refused for user endpoints where credentials will be sent — the warning language states the reason, not just "invalid".
- Rust classification is the apply-time authority; UI feedback is advisory. (Enforcement wiring completes at Save & Apply, T33.)

## Tests (evidence)
- Rust **23/23** (5 new: remote-ok, insecure schemes, local hosts, garbage invalid, tagged serialization).
- Frontend **36/36** (5 new mirror/classification tests + live-warning, local-note, clean-https UI tests).
- clippy/fmt clean; frontend gate green.

## Acceptance criteria
- [x] Editable custom Base URL with live validation
- [x] Secure-protocol warnings with a11y semantics
- [x] Rust-side authority + TS mirror in sync
