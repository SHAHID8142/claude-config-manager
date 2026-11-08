//! V2-19: backward-compatible migration of legacy per-provider secrets.
//!
//! Before V2 a provider had exactly one credential at
//! `secrets/<provider-id>.secret`. Multi-credential support must not orphan
//! those keys, and must not rewrite a single byte of them either. This module
//! therefore only *derives* index records: the secret file stays where it is,
//! and the synthesized record's id is chosen so that the legacy file is the one
//! that record resolves to.
//!
//! Properties the tests pin down:
//! * a provider with an existing record is never migrated twice (idempotent);
//! * a legacy provider gets one enabled record at priority 0, labelled
//!   "Existing key", so the UI can show it immediately;
//! * `last4` is left empty rather than guessed — the migration does not read
//!   secret material at all;
//! * invalid directory entries (a file called `.secret`, a stray name) are
//!   ignored, never turned into records.

use crate::credential_index::{CredentialIndex, CredentialRecord, validate_credential_id};

/// Legacy record id for a provider: `<provider>-legacy`, which is itself a
/// valid credential id and cannot collide with a generated id (`<provider>-<n>`).
pub fn legacy_credential_id(provider_id: &str) -> String {
    format!("{provider_id}-legacy")
}

/// Providers that currently have a legacy `secrets/<provider>.secret` file but
/// no record in the index.
///
/// `legacy_secret_stems` must come from a directory listing of stems, not from
/// reading file contents; this function never opens a secret.
pub fn pending_legacy_providers(
    index: &CredentialIndex,
    legacy_secret_stems: &[String],
) -> Vec<String> {
    let mut out: Vec<String> = legacy_secret_stems
        .iter()
        .filter(|stem| crate::credential_handoff::validate_provider_id(stem).is_ok())
        .filter(|stem| index.for_provider(stem).is_empty())
        .cloned()
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Index with one synthesized record per pending legacy provider.
///
/// Pure: the caller decides whether to persist the result, and the secret files
/// are untouched either way.
pub fn migrate_legacy(
    index: &CredentialIndex,
    legacy_secret_stems: &[String],
    now_rfc3339: &str,
) -> CredentialIndex {
    let mut next = index.clone();
    for provider in pending_legacy_providers(index, legacy_secret_stems) {
        let id = legacy_credential_id(&provider);
        if validate_credential_id(&id).is_err() || next.get(&id).is_some() {
            // A provider id long enough to break the derived id is skipped
            // rather than truncated: a truncated id could point at the wrong
            // secret file.
            continue;
        }
        let record = CredentialRecord {
            id,
            provider_id: provider,
            label: "Existing key".into(),
            // Deliberately unknown: migration must not read the key.
            last4: String::new(),
            priority: 0,
            enabled: true,
            created_at: now_rfc3339.into(),
        };
        if next.insert(record).is_err() {
            // Validation failure means the index stays as it was; a bad
            // migration must never make the app unable to load its own state.
            return index.clone();
        }
    }
    next
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credential_index::INDEX_VERSION;

    fn empty() -> CredentialIndex {
        CredentialIndex {
            version: INDEX_VERSION,
            credentials: vec![],
        }
    }

    fn record(id: &str, provider: &str) -> CredentialRecord {
        CredentialRecord {
            id: id.into(),
            provider_id: provider.into(),
            label: "work".into(),
            last4: "ab12".into(),
            priority: 1,
            enabled: true,
            created_at: "2026-11-01T00:00:00Z".into(),
        }
    }

    #[test]
    fn migrates_each_legacy_provider_once() {
        let legacy = vec!["openrouter".to_string(), "anthropic".to_string()];
        let migrated = migrate_legacy(&empty(), &legacy, "2026-11-08T00:00:00Z");
        assert_eq!(migrated.credentials.len(), 2);
        let ids: Vec<&str> = migrated.credentials.iter().map(|c| c.id.as_str()).collect();
        // Pending providers are sorted, so the file order is deterministic.
        assert_eq!(ids, ["anthropic-legacy", "openrouter-legacy"]);
        for c in &migrated.credentials {
            assert_eq!(c.priority, 0);
            assert!(c.enabled);
            assert_eq!(c.label, "Existing key");
            // Never guessed from the file.
            assert!(c.last4.is_empty());
        }
        // Idempotent: running again against the migrated index adds nothing.
        let again = migrate_legacy(&migrated, &legacy, "2026-11-09T00:00:00Z");
        assert_eq!(again, migrated);
    }

    #[test]
    fn never_overwrites_a_provider_that_already_has_a_credential() {
        let mut index = empty();
        index
            .insert(record("openrouter-1", "openrouter"))
            .expect("insert");
        let migrated = migrate_legacy(&index, &["openrouter".into()], "2026-11-08T00:00:00Z");
        assert_eq!(migrated, index);
        assert_eq!(migrated.for_provider("openrouter").len(), 1);
    }

    #[test]
    fn ignores_stray_and_unsafe_directory_entries() {
        let legacy = vec![
            "openrouter".to_string(),
            "".to_string(),
            "../escape".to_string(),
            "Bad Name".to_string(),
            "openrouter".to_string(), // duplicate listing
        ];
        let migrated = migrate_legacy(&empty(), &legacy, "2026-11-08T00:00:00Z");
        assert_eq!(migrated.credentials.len(), 1);
        assert_eq!(migrated.credentials[0].id, "openrouter-legacy");
    }

    #[test]
    fn pending_list_is_sorted_and_deduped() {
        let pending = pending_legacy_providers(
            &empty(),
            &["zai".into(), "anthropic".into(), "anthropic".into()],
        );
        assert_eq!(pending, ["anthropic", "zai"]);
    }

    #[test]
    fn a_provider_id_too_long_for_the_derived_id_is_skipped() {
        // 60 chars + "-legacy" = 67 > 64, so the derived id is invalid.
        let long = "a".repeat(60);
        assert!(validate_credential_id(&legacy_credential_id(&long)).is_err());
        let migrated = migrate_legacy(
            &empty(),
            std::slice::from_ref(&long),
            "2026-11-08T00:00:00Z",
        );
        assert!(migrated.credentials.is_empty());
    }
}
