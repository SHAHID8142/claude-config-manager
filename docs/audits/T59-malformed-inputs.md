# T59 — Malformed URL / JSON / model-metadata / command-injection regression suite

**Date:** 2026-11-08 · **Result:** ✅ PASS — suite written, **5 real defects found and fixed** ·
**Artifact:** `src-tauri/tests/malformed_inputs.rs` (14 cases) + fixes in `provider_catalog.rs`,
`credential_handoff.rs`, `model_discovery.rs`

## Suite contents (`cargo test --test malformed_inputs`)
| Area | Cases |
|---|---|
| Malformed URLs | empty/blank, no scheme, bare host, `https://`, embedded space, control chars + newline, `javascript:`, `data:`, `file:`, `ftp:`, protocol-relative `//`, `user:pass@`, quotes/angle brackets, plain `http` |
| Malformed JSON | empty, truncated, array root, scalar root, trailing garbage, control chars, comments, `true`; plus missing-file vs malformed distinction and "file must stay byte-identical" on every failure |
| Hostile model metadata | non-string/null/empty/whitespace ids dropped; catalog that is not a list refused; non-object entries skipped without killing the list; payload cannot dictate `source` or `capabilityStatus` (T42 regression) |
| Command injection | 17 provider-id shapes (`;`, `$( )`, backticks, pipes, quotes, `../`, newline, NUL, leading `-`) refused and **no helper script generated**; valid id produces a script referencing only that id; model ids proven never to reach a shell |

## Defects found by writing the suite (each fixed with the failing test as guard)
1. **Credential embedded in the base URL was accepted.** `https://user:pass@example.com` classified
   `RemoteOk` → a live credential would have been written to `settings.json` (plaintext, and copied into
   every backup). Now refused with an explicit reason.
2. **Whitespace/newline in a URL were silently normalised.** The URL parser strips tabs/newlines, so
   `https://example.com\n.evil.com` became a **different host** than the one displayed. Control/whitespace
   characters are now refused *before* parsing.
3. **Non-HTTP schemes were misreported as "Insecure".** `javascript:`, `data:`, `file:`, `ftp:` are not
   endpoints at all; telling the user "use https" for them is wrong advice. They are now `Invalid` with the
   scheme named.
4. **Provider ids could start with `-`** (`-rf` passed the old `[a-z0-9-]+` rule). Any consumer that
   forwards the id would read it as a flag. Ids must now start with a letter/digit.
5. **Whitespace-only catalog ids were kept** (`"id":"   "` produced a blank, selectable row that then
   failed at apply time). Ids are trimmed and empty-after-trim entries dropped.

## Bugs in the tests themselves (also corrected, honestly noted)
- I initially asserted `model$(whoami)` must be rejected; the documented rule is *printable ASCII*, so it is
  accepted — safe only because no code path interpolates a model id into a command. The test now asserts
  both halves instead of an untrue one.
- I asserted a numeric-entry catalog (`[1,2,3]`) must error; by design a junk entry is skipped rather than
  failing the whole catalog. Test corrected to assert that behaviour plus a mixed list.
- `"model;rm -rf /"` contains a space and was therefore already invalid — my "hostile but allowed" example
  was wrong; replaced with `model;rm-rf`.

## Hardening of the toolchain gate
`cargo clippy --all-targets -- -D warnings` was failing on 8 pre-existing lints in **test** code (deferred
since T33). All fixed (unused import/mut, `contains` over `iter().any`, `assert_eq!(x, true/false)` →
`assert!(x)`). The stricter gate now passes, so future test code must meet it.

## Tests (evidence)
- `cargo test --test malformed_inputs` → **14 passed**
- `cargo test` → **156 passed, 2 ignored + 7 (adversarial_paths) + 14 (malformed_inputs) = 179**
- `cargo fmt --check` ✓ · `cargo clippy --all-targets -- -D warnings` ✓ (0 errors)
- Frontend untouched by this task (110/110 green at T45/T46).

## Acceptance criteria
- [x] Malformed URL regression coverage incl. scheme, credential, whitespace, injection shapes
- [x] Malformed JSON never repaired, never rewritten, byte-identical on failure
- [x] Hostile model metadata cannot dictate provenance/capability and cannot disable the list
- [x] Command-injection coverage for the only shell-interpolated value (provider id)
- [x] Findings fixed, not just documented

## Known limitations
- Duplicate JSON keys are still normalised by the parser (last value wins) rather than refused; our writes
  never create duplicates, but a hand-edited file could carry one. Tracked as a follow-up hardening item,
  disclosed rather than silently accepted.
- Percent-encoded path segments (`%2e%2e`) are not decoded by the classifier; they reach the provider as
  opaque paths and grant no local access in this app.
- The suite targets the local trust boundary (files, ids, catalog payloads). Network-layer attacks (DNS
  rebinding, TLS interception) are out of scope here and belong to T60.
