# Gap closure register

Created after the question "did you audit every aspect?". Each entry is a gap that existed at `54d2ba5`,
closed in its own commit, with the evidence. Gaps that cannot be closed here are marked **BLOCKED** with
the reason — never silently dropped.

| Gap | Description | Status |
|---|---|---|
| G1 | Read-back failure / rollback branch had no test | ✅ closed |
| G2 | Credential-redirect safety relied on an unasserted `ureq` default | ✅ closed |
| G3 | No property-based testing of the merge contract | ✅ closed |
| G4 | No single-instance lock: two applies can race | ✅ closed |
| G5 | No size/scale limits (huge settings file, huge catalog body) | ✅ closed |
| G6 | Settings JSON dialect assumed strict (JSON5/comments would be refused) | ✅ closed (verified, not changed) |
| G7 | Real GUI / VoiceOver / contrast / clean-Mac launch | **BLOCKED** (no signed build, no machine evidence) |
| G8 | Bridge mode fidelity (streaming, tools, vision, reasoning) | **BLOCKED** (feature absent, T47–T58) |
| G9 | CI pipeline itself never executed | **BLOCKED** (GitHub billing) |
| G10 | TLS/proxy/certificate-error posture | ✅ closed |

---

## G1 — read-back failure and rollback

**Defect class:** the safest code path in the app (verify-after-write, restore-on-mismatch) was implemented
but never executed by a test.

**Change:** `write_settings_atomic` now delegates to `write_settings_atomic_verified(path, doc, mtime,
verify)`. The verifier is a parameter; the *rollback logic is the same code path* — it was extracted, not
duplicated, so the test cannot pass against a re-implementation. Default verifier = re-read + parse + compare.

Two honest behaviours the tests pinned:
- **Existing file:** failure restores the previous bytes **exactly** (including whitespace), keeps mode
  `0600`, and leaves **no temp file**.
- **First run:** there is nothing to restore, so the created file is **removed** — a failed first write leaves
  no file at all rather than an empty one that Claude Code would then read as "no configuration".

Evidence: `failed_verification_restores_the_original_bytes_exactly`,
`failed_verification_on_first_run_leaves_no_file` (both in `config_writer.rs`).

---

## G2 — credential forwarding across redirects

**Gap class:** safety depended on an unasserted dependency default (`ureq` ships `max_redirects: 10` with
`redirect_auth_headers: Never`). Correct today, silently regrettable after any upgrade.

**Change:** the discovery agent now states the policy itself:
`max_redirects(0)`, `max_redirects_will_error(true)`, `redirect_auth_headers(Never)`. With `0`, `ureq`
returns the 3xx instead of following it, and a 3xx is now surfaced as an explicit `HttpStatus(302)` rather
than the misleading "response unusable" parse error it produced before.

**Evidence:** `a_redirecting_catalog_is_refused_and_the_credential_is_not_forwarded` — a loopback server
answers `302` pointing at a second server that records every request it receives. Assertions: the error is
`HttpStatus(302)`, the target was **never contacted**, and the credential string never appears in any
forwarded request.

**Residual:** a `302` is now a hard error, so a provider that legitimately redirects its catalog (e.g. an
`http→https` upgrade) will show an error instead of "just working". That is the deliberate trade — an
explicit error beats a credential that quietly travels somewhere else.

---

## G3 — property-based testing of the merge contract

**Gap class:** every adversarial case was hand-written by me after reading the code — coverage limited by my
own imagination. No fuzzing or property tests existed.

**Change:** `proptest` added as a dev-dependency; `src-tauri/tests/merge_properties.rs` asserts six invariants
over generated documents (400 cases per property in CI runs, verified up to 2 000 locally):

| # | Invariant |
|---|---|
| P1 | `apply_desired` is total — arbitrary JSON in, `Ok`/`Err` out, never a panic |
| P2 | idempotent — applying the same intent twice equals applying it once |
| P3 | ownership — unmanaged top-level and `env` keys survive unchanged |
| P4 | written env values are exactly the requested strings, never `null` |
| P5 | credential-shaped model ids are refused, never written in plaintext |
| P6 | accepted output serialises and re-parses to the same document |

**What the properties found (honest record):** five failures on first run, **none of them app defects** —
and the way they failed is the useful part:
1. `serde_json` does not round-trip extreme f64 exponents exactly (`3.942350228396264e268` reparses as
   `...2635e268`). Upstream behaviour; the number domain was narrowed to realistic settings values and the
   limitation is recorded here rather than silently avoided.
2. `is_secret_shaped` has a **16-character floor**, so a 12-char `ghp_` string is not flagged. Correct by
   design; the generator's token lengths were unrealistic, and the floor is now documented.
3. Two assertion bugs of mine: asserting that *pre-existing* user env garbage must be a string (it must not be
   "fixed" — preserving it is the promise), and a credential generator producing bare prefixes.

**Disclosed limits:**
- Numbers with |exponent| > ~22 do not survive a serialise/parse round-trip in `serde_json`; settings never
  contain them.
- Secret detection needs ≥ 16 characters and a known prefix: `sk-ant-` alone is not detected. No false
  negatives for real provider key formats, which are all far longer.

---

## G4 — concurrent applies

**Gap class:** nothing prevented two app instances (or two windows) from running the read-merge-write
pipeline at once. The mtime check is check-then-act, so two writers could both pass it.

**Change:** new `src-tauri/src/apply_lock.rs`, acquired at the top of `apply_with_roles` and released by
`Drop`:
- `create_new` (O_EXCL) — the acquisition itself is atomic;
- file holds `pid` + a per-acquisition nonce, mode `0600`;
- a lock is reclaimed only if its owner is gone **and** it is older than a 5 s grace window;
- hard staleness at 120 s, so a crash can never wedge the app permanently.

**Two bugs this found in my own first implementation** (both would have been shipped otherwise):
1. *Cross-delete by pid.* Ownership was decided by pid, so two guards inside one process looked identical and
   the loser's `Drop` deleted the winner's lock. Fixed with a nonce token compared on release.
2. *Empty-file race.* `create_new` publishes the file before the pid is written; a second process read an
   empty file, concluded "owner dead", and stole the lock — the concurrency test failed with
   `write verification failed` under parallel load. Fixed by failing closed inside the grace window.

**Evidence:** 8 unit tests (`an_empty_but_fresh_lock_is_held_not_stolen`,
`the_holder_is_not_deleted_by_a_stale_reclaimer_that_lost`, `a_lock_from_a_dead_process_is_reclaimed`, …) plus
`concurrent_applies_are_serialized_and_never_corrupt_the_file` — 8 threads over one settings file: at least
one succeeds, every other failure must carry the lock message, the file stays valid JSON with the user's
unrelated env key intact, and no lock file remains. Full suite re-run 5× with no flakes.

**Residual:**
- The lock is advisory and local to one user account; it is not an OS-level file lock, so an external editor
  writing the file is still only caught by the mtime check.
- A crash within the 5 s window means a user-visible "another apply is running" for up to 5 s. Accepted:
  better a short wait than two writers.
- The UI does not yet grey out the button while an apply is in flight (the backend refuses correctly).

---

## G5 — size and scale limits

**Gap class:** no bound on what the app would try to load. A `settings.json` of any size was read fully into
memory before being judged; only the catalog HTTP body had a cap.

**Change:**
- `config_reader::SETTINGS_MAX_BYTES = 8 MiB`, checked from metadata **before** reading, in both the read path
  and the write path (so a huge file can never be "backed up" into memory).
- New `ConfigError::TooLarge { size, limit }` — reporting the real size, while keeping the existing promise
  that error text never contains file *content*.
- Discovery body cap (5 MiB) now has a test that actually streams past it.

**Evidence:**
- `oversized_settings_file_is_refused_with_the_size_in_the_message` (8 MiB + 1 → `TooLarge`, both numbers in
  the message)
- `deeply_nested_json_is_rejected_not_a_stack_overflow` (5 000 nested arrays → parse error, no crash)
- `oversized_catalog_body_is_cut_off_at_the_cap` (server streams past the cap and never finishes → explicit
  "too large")

**Honest note:** while writing the oversized-file test I discovered the size message was being swallowed —
`MalformedJson`'s `Display` deliberately hides its payload, so the first version silently degraded to "invalid
JSON". That is why the dedicated variant exists; the test asserts the structured variant, not just the text.

**Residual:** 8 MiB is a judgement call; a legitimate file that big would be refused (with a clear message)
rather than silently truncated. The limit is a constant, not user-tunable.

---

## G6 — JSON dialect assumption

**Gap class:** the reader assumed strict JSON. If Claude Code accepted comments or trailing commas, we would
refuse to manage a perfectly valid file.

**Resolution: the assumption is now *verified*, not assumed.** The official settings documentation says, of
the annotated examples: *"Claude Code doesn't accept comments in a settings file, so copy from the first
tab."* ([code.claude.com/docs/en/settings-example](https://code.claude.com/docs/en/settings-example); the
JSONC feature request, [claude-code#17968](https://github.com/anthropics/claude-code/issues/17968), is still
open). Strict parsing is therefore correct behaviour, and a lenient parser would have been a data-loss bug —
we would have "accepted" a file Claude Code itself then rejects.

**Change is UX, not semantics:** invalid JSON that is recognisably one of those dialects now produces
`ConfigError::MalformedJsonHint` with a fixed hint ("comments are not allowed in settings.json", "trailing
comma") instead of a bare "invalid JSON". The detector scans **outside string literals only**, so
`"https://example.com/x"` never looks like a comment.

**Evidence:** `comments_are_refused_with_a_cause_not_a_generic_error`,
`trailing_comma_is_refused_and_named`,
`comment_markers_inside_strings_are_not_mistaken_for_comments` (false-positive guard, including
`// tail` and a `/* … */` inside a string).

**Pre-existing tests updated, disclosed:** `malformed_json_reports_position_without_content_leak` used a
trailing comma as its "malformed" sample, which now yields the named hint; it was switched to a genuine
syntax error so it still tests position reporting, and `json_comments_are_rejected_strict` now accepts either
variant. Neither test's intent was weakened — the rejection contract is unchanged.

---

## G10 — TLS, proxies and certificate posture

**Gap class:** no work at all on transport trust. The build used `ureq`'s bundled webpki roots, ignored
`HTTPS_PROXY`, and nothing asserted that verification was on.

**Change:**
- `platform-verifier` feature enabled and `RootCerts::PlatformVerifier` set explicitly → trust follows the
  **macOS system store**, so a root installed by the user's organisation (MDM) works, while verification
  stays on.
- Proxy support wired through `Proxy::try_from_env()` (honours `HTTP(S)_PROXY` and `NO_PROXY`), injectable as
  a parameter so it can be tested without mutating process-global environment variables.

**Evidence:**
- `requests_go_through_the_configured_proxy` — a loopback fake proxy performs the `CONNECT` tunnel and
  receives the tunnelled `GET /v1/models`; the origin host is unresolvable (`.invalid`), so the request can
  only have succeeded through the proxy.
- `live_self_signed_certificate_is_refused` (**`#[ignore]`, live network, run and passing**) — a self-signed
  certificate is rejected, proving verification is not disabled.
- Live catalog tests re-run with the new TLS config: NVIDIA NIM still returns its catalog.

**Two things this taught the test author:** `ureq` tunnels even plain-HTTP origins with `CONNECT`, and closing
a fake proxy socket before the client finishes writing shows up as `Broken pipe` — both fixed in the fixture,
not worked around in production code.

**Residual:**
- A corporate TLS-inspecting proxy that re-signs traffic will only work if its root is installed in the system
  store; otherwise the user sees a transport error (accurate, but terse).
- No certificate pinning (deliberate: providers rotate certificates and endpoints).
- SOCKS proxies: `socks-proxy` feature is not enabled, so `ALL_PROXY=socks5://` is ignored.
