//! Credential index (V2-18): the metadata half of a multi-credential store.
//!
//! Why a separate file: V2 needs several credentials per provider (different
//! keys, different quotas) without ever putting key material in a JSON file.
//! The index holds **only metadata** — id, provider, label, last four
//! characters, order, enabled flag — and the secret itself stays in
//! `secrets/<credential-id>.secret` (0600, written by `FileSecretStore`).
//!
//! Backward compatibility (V2-19): a provider with no index entry keeps using
//! the legacy `secrets/<provider-id>.secret`, so an installed app never loses
//! the credential it already has.
//!
//! What this module deliberately does NOT store: the key, a hash of the key, or
//! anything derived from it beyond the last four characters that the UI needs
//! to tell two keys apart.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Current on-disk schema version. Unknown versions are refused, never guessed.
pub const INDEX_VERSION: u32 = 1;

const MAX_LABEL: usize = 64;
const MAX_CREDENTIALS: usize = 200;

/// Validation failures. Messages never include key material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexError {
    /// Not a v1 index, or malformed JSON.
    BadFormat(String),
    /// A record failed a field-level rule.
    Invalid(String),
    /// I/O while reading or writing.
    Io(String),
}

impl std::fmt::Display for IndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexError::BadFormat(m) | IndexError::Invalid(m) | IndexError::Io(m) => f.write_str(m),
        }
    }
}

/// One credential's metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CredentialRecord {
    /// Stable id, also the secret file stem: `[a-z0-9-]{1,64}`.
    pub id: String,
    /// Provider this credential belongs to (`openrouter`, `anthropic`, …).
    pub provider_id: String,
    /// Human label ("work key", "free tier"). Never contains key material.
    pub label: String,
    /// Last four characters, shown masked. Empty when the key is shorter.
    pub last4: String,
    /// Lower runs first in a failover chain.
    pub priority: u32,
    /// A disabled credential is never selected or routed to.
    pub enabled: bool,
    /// RFC 3339 timestamp of creation, for stable display order.
    pub created_at: String,
}

/// The whole index file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CredentialIndex {
    pub version: u32,
    pub credentials: Vec<CredentialRecord>,
}

impl Default for CredentialIndex {
    fn default() -> Self {
        Self {
            version: INDEX_VERSION,
            credentials: Vec::new(),
        }
    }
}

impl CredentialIndex {
    /// Parse and validate. Anything off is refused as a whole, so a partially
    /// understood file can never drive a write.
    pub fn parse(raw: &str) -> Result<Self, IndexError> {
        let index: CredentialIndex = serde_json::from_str(raw)
            .map_err(|e| IndexError::BadFormat(format!("credentials.json: {e}")))?;
        index.validate()?;
        Ok(index)
    }

    pub fn to_json(&self) -> Result<String, IndexError> {
        self.validate()?;
        serde_json::to_string_pretty(self)
            .map(|mut s| {
                s.push('\n');
                s
            })
            .map_err(|e| IndexError::BadFormat(e.to_string()))
    }

    pub fn validate(&self) -> Result<(), IndexError> {
        if self.version != INDEX_VERSION {
            return Err(IndexError::BadFormat(format!(
                "unsupported credentials.json version {} (this build writes {INDEX_VERSION})",
                self.version
            )));
        }
        if self.credentials.len() > MAX_CREDENTIALS {
            return Err(IndexError::Invalid(format!(
                "too many credentials ({}, max {MAX_CREDENTIALS})",
                self.credentials.len()
            )));
        }
        let mut seen: Vec<&str> = Vec::with_capacity(self.credentials.len());
        for c in &self.credentials {
            validate_credential_id(&c.id)?;
            crate::credential_handoff::validate_provider_id(&c.provider_id).map_err(|_| {
                IndexError::Invalid(format!("credential {}: invalid provider", c.id))
            })?;
            if seen.contains(&c.id.as_str()) {
                return Err(IndexError::Invalid(format!(
                    "duplicate credential id {}",
                    c.id
                )));
            }
            seen.push(&c.id);
            if c.label.trim().is_empty() || c.label.chars().count() > MAX_LABEL {
                return Err(IndexError::Invalid(format!(
                    "credential {}: label must be 1–{MAX_LABEL} characters",
                    c.id
                )));
            }
            if c.label.chars().any(|ch| ch.is_control()) {
                return Err(IndexError::Invalid(format!(
                    "credential {}: label contains control characters",
                    c.id
                )));
            }
            if c.last4.chars().count() > 4 || !c.last4.chars().all(|ch| ch.is_ascii_graphic()) {
                return Err(IndexError::Invalid(format!(
                    "credential {}: last4 must be at most 4 printable ASCII characters",
                    c.id
                )));
            }
            if c.created_at.trim().is_empty() {
                return Err(IndexError::Invalid(format!(
                    "credential {}: missing created_at",
                    c.id
                )));
            }
        }
        Ok(())
    }

    /// Records for one provider in failover order (priority, then id).
    pub fn for_provider(&self, provider_id: &str) -> Vec<&CredentialRecord> {
        let mut out: Vec<&CredentialRecord> = self
            .credentials
            .iter()
            .filter(|c| c.provider_id == provider_id)
            .collect();
        out.sort_by(|a, b| a.priority.cmp(&b.priority).then_with(|| a.id.cmp(&b.id)));
        out
    }

    /// Enabled records for one provider, in failover order.
    pub fn active_for_provider(&self, provider_id: &str) -> Vec<&CredentialRecord> {
        self.for_provider(provider_id)
            .into_iter()
            .filter(|c| c.enabled)
            .collect()
    }

    pub fn get(&self, id: &str) -> Option<&CredentialRecord> {
        self.credentials.iter().find(|c| c.id == id)
    }

    /// Insert, refusing duplicate ids (the caller picks a fresh id).
    pub fn insert(&mut self, record: CredentialRecord) -> Result<(), IndexError> {
        if self.get(&record.id).is_some() {
            return Err(IndexError::Invalid(format!(
                "duplicate credential id {}",
                record.id
            )));
        }
        self.credentials.push(record);
        self.validate()
    }

    /// Remove a record; returns whether it existed.
    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.credentials.len();
        self.credentials.retain(|c| c.id != id);
        before != self.credentials.len()
    }

    /// Move one credential to the front of its provider's chain while keeping
    /// the rest in their relative order (V2-22 priority editing).
    pub fn promote(&mut self, id: &str) -> bool {
        let Some(target) = self.get(id).cloned() else {
            return false;
        };
        let mut chain: Vec<CredentialRecord> = self
            .for_provider(&target.provider_id)
            .into_iter()
            .cloned()
            .collect();
        chain.retain(|c| c.id != id);
        chain.insert(0, target);
        for (i, c) in chain.iter().enumerate() {
            if let Some(slot) = self.credentials.iter_mut().find(|x| x.id == c.id) {
                slot.priority = i as u32;
            }
        }
        true
    }
}

/// `[a-z0-9-]{1,64}`: safe as a filename and as a helper argument.
pub fn validate_credential_id(id: &str) -> Result<(), IndexError> {
    let ok = !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-');
    if ok {
        Ok(())
    } else {
        Err(IndexError::Invalid(
            "credential id must match [a-z0-9-]{1,64}".into(),
        ))
    }
}

/// Last four characters of a key, for the masked label. Never the whole key.
pub fn last4_of(secret: &str) -> String {
    let chars: Vec<char> = secret.trim().chars().collect();
    if chars.len() <= 4 {
        return String::new();
    }
    chars[chars.len() - 4..].iter().collect()
}

/// `<home>/.claude-config-manager/credentials.json`
pub fn index_path(home: &Path) -> PathBuf {
    home.join(".claude-config-manager").join("credentials.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: &str, priority: u32) -> CredentialRecord {
        CredentialRecord {
            id: id.into(),
            provider_id: "openrouter".into(),
            label: "work".into(),
            last4: "ab12".into(),
            priority,
            enabled: true,
            created_at: "2026-11-08T00:00:00Z".into(),
        }
    }

    fn index(records: Vec<CredentialRecord>) -> CredentialIndex {
        CredentialIndex {
            version: INDEX_VERSION,
            credentials: records,
        }
    }

    #[test]
    fn round_trips_through_json_without_key_material() {
        let mut idx = index(vec![record("work-1", 0)]);
        idx.insert(record("free-2", 1)).expect("inserts");
        let json = idx.to_json().expect("serializes");
        // The index may only ever carry metadata.
        assert!(!json.contains("sk-"));
        assert!(json.contains("\"last4\": \"ab12\""));
        assert_eq!(CredentialIndex::parse(&json).expect("parses"), idx);
    }

    #[test]
    fn refuses_unknown_versions_and_unknown_fields() {
        let err =
            CredentialIndex::parse(r#"{"version":99,"credentials":[]}"#).expect_err("version");
        assert!(err.to_string().contains("unsupported"));
        // deny_unknown_fields: a field this build does not understand is a hard
        // error, so a future format cannot be silently half-read.
        let err = CredentialIndex::parse(r#"{"version":1,"credentials":[],"extra":true}"#)
            .expect_err("unknown field");
        assert!(matches!(err, IndexError::BadFormat(_)));
    }

    #[test]
    fn rejects_unsafe_ids_duplicates_and_oversized_labels() {
        let idx = index(vec![record("Ok_ID", 0)]);
        assert!(matches!(idx.validate(), Err(IndexError::Invalid(_))));

        let mut idx = index(vec![record("a", 0)]);
        assert!(idx.insert(record("a", 1)).is_err());
        // ...but the rejected record must not have been added.
        assert_eq!(idx.credentials.len(), 1);

        let mut bad = record("b", 0);
        bad.label = "x".repeat(MAX_LABEL + 1);
        let mut idx = index(vec![]);
        assert!(matches!(idx.insert(bad), Err(IndexError::Invalid(_))));

        let mut ctrl = record("c", 0);
        ctrl.label = "line\nbreak".into();
        let mut idx = index(vec![]);
        assert!(matches!(idx.insert(ctrl), Err(IndexError::Invalid(_))));
    }

    #[test]
    fn orders_by_priority_then_id_and_filters_disabled() {
        let mut second = record("second", 5);
        second.priority = 1;
        let mut off = record("off", 0);
        off.enabled = false;
        let idx = index(vec![second, record("first", 0), off]);
        let ordered: Vec<&str> = idx
            .for_provider("openrouter")
            .iter()
            .map(|c| c.id.as_str())
            .collect();
        // Equal priorities tie-break on id, so the order is stable across loads.
        assert_eq!(ordered, ["first", "off", "second"]);
        let active: Vec<&str> = idx
            .active_for_provider("openrouter")
            .iter()
            .map(|c| c.id.as_str())
            .collect();
        assert_eq!(active, ["first", "second"]);
        assert!(idx.for_provider("anthropic").is_empty());
    }

    #[test]
    fn promote_moves_only_within_the_provider_chain() {
        let mut other = record("other", 0);
        other.provider_id = "anthropic".into();
        let mut idx = index(vec![record("a", 0), record("b", 1), record("c", 2), other]);
        assert!(idx.promote("c"));
        let order: Vec<&str> = idx
            .for_provider("openrouter")
            .iter()
            .map(|c| c.id.as_str())
            .collect();
        assert_eq!(order, ["c", "a", "b"]);
        assert_eq!(idx.get("other").expect("kept").priority, 0);
    }

    #[test]
    fn last4_never_leaks_short_secrets() {
        assert_eq!(last4_of("sk-or-v1-abcdef"), "cdef");
        // A 4-character key would be fully revealed by a naive suffix.
        assert_eq!(last4_of("abcd"), "");
        assert_eq!(last4_of("  "), "");
    }

    #[test]
    fn index_path_is_space_free_for_helper_use() {
        let p = index_path(Path::new("/Users/x"));
        assert!(!p.to_string_lossy().contains(' '));
    }
}
