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
