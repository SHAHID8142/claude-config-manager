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
| G5 | No size/scale limits (huge settings file, huge catalog body) | pending |
| G6 | Settings JSON dialect assumed strict (JSON5/comments would be refused) | pending |
| G7 | Real GUI / VoiceOver / contrast / clean-Mac launch | **BLOCKED** (no signed build, no machine evidence) |
| G8 | Bridge mode fidelity (streaming, tools, vision, reasoning) | **BLOCKED** (feature absent, T47–T58) |
| G9 | CI pipeline itself never executed | **BLOCKED** (GitHub billing) |
| G10 | TLS/proxy/certificate-error posture | pending |

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
