# V2-22 — credential priority and enable/disable

Shipped with V2-20/V2-21, pinned by tests rather than a new subsystem:

* **Order** — `CredentialIndex::for_provider` sorts by `priority`, then `id`, so
  the chain is stable across reloads. `add` assigns the next free priority.
* **Promote** — `promote(id)` moves one credential to the front of *its own
  provider's* chain and renumbers only that chain (a credential from another
  provider is never reordered). Exposed as the ↑ button in the panel.
* **Disable** — `set_enabled(id, false)` keeps the credential and its secret but
  removes it from `active_for_provider`, i.e. from what failover will try. The
  panel's row switch does exactly this and never deletes anything.
* **Delete is separate** — only an explicit trash action removes a credential,
  and a migrated legacy key has no trash action at all.

Evidence: `credential_index::tests::orders_by_priority_then_id_and_filters_disabled`,
`promote_moves_only_within_the_provider_chain`,
`credential_vault::tests::disabling_hides_a_credential_from_the_active_chain_but_keeps_it`,
`persists_across_instances_and_keeps_chain_order`; UI tests
`disables, promotes and removes by explicit action only`.
