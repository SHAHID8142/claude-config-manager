# Content and data model

No CMS is needed: this is a local desktop configuration utility.

Current persisted objects: Claude Code global settings; credential index v1;
0600 secret files; routing v1 flat targets; helper process state/token;
LaunchAgent; settings backups; optional bridge consent and installation.
React draft state and localStorage navigation are separate from these files.

Proposed user model: ordered provider profiles, each with its endpoint/protocol,
explicit model mapping, ordered credential references, enabled state, and
per-key health/cooldown. Secrets remain in the existing private store.
Migration must preserve existing legacy keys and routing order, and preserve
the ability to restore the previous configuration format.

User-facing help inventory: `help-inventory.md`. Detailed workflow and acceptance
criteria: `plan.md`. Engineering evidence: `../docs/audits/2026-11-17-project-review.md`.


Save-button follow-up: provider support is distinct from key storage/catalog
browsing. An unavailable provider shows a persistent explanation and action
reason; key storage does not imply usable inference or successful key checking.


1.0.4 supersedes the unavailable NVIDIA state. NVIDIA has a built-in connection
with the existing helper; key and model are required for Save & Apply. Successful
Save activates NVIDIA first and retains other providers. Unsupported features
remain explicit; catalog availability does not prove key validity or inference.


1.0.5: valid provider drafts persist separately in bounded WebKit localStorage;
no key input stored. Completed custom endpoint/model drafts are scoped by endpoint
hash. Private managed-operation before-images live in apply-journal.json; explicit
recovery snapshots may contain key copies. Activity retains100 fixed-category,
provider/timestamp events, never prompts/responses/endpoints/key values/key labels.

1.0.7: helper health adds optional version; runtime.lock is a persistent empty
private regular file holding an OS exclusive lease for the helper lifetime. It is
not an age-based transaction lock. System text is translated to one initial
NVIDIA instruction with active turn scopes; dynamic tool-state changes are rejected.
