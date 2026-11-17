//! V2-20: multi-credential backend.
//!
//! One provider can own several credentials (different keys, different quotas).
//! This module is the only writer of `credentials.json` and the only place that
//! maps a credential id to a secret file:
//!
//! * metadata → `credentials.json` (0600, atomic temp + fsync + rename)
//! * secret   → `secrets/<credential-id>.secret` (0600, via `FileSecretStore`)
//!
//! Invariants kept by every mutator:
//! * the index on disk is only ever replaced whole, and only after it validates;
//! * a failing secret write leaves the index unchanged (no record without a
//!   key, no key without a record);
//! * removing a credential deletes its secret **and** its record, and is
//!   idempotent;
//! * legacy `<provider>.secret` files are never read by migration, so an
//!   upgraded install keeps working without a rewrite.

use crate::credential_index::{
    CredentialIndex, CredentialRecord, IndexError, index_path, last4_of, validate_credential_id,
};
use crate::credential_migration::migrate_legacy;
use crate::credential_service::CredentialService;
use crate::file_secret_store::FileSecretStore;
use std::fs;
#[cfg(test)]
use std::io::Write;
#[cfg(test)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum VaultError {
    Index(IndexError),
    Secret(String),
    Io(String),
    Invalid(String),
}

impl std::fmt::Display for VaultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VaultError::Index(e) => write!(f, "{e}"),
            VaultError::Secret(m) | VaultError::Io(m) | VaultError::Invalid(m) => f.write_str(m),
        }
    }
}

impl From<IndexError> for VaultError {
    fn from(e: IndexError) -> Self {
        VaultError::Index(e)
    }
}

pub struct CredentialVault {
    root: PathBuf,
    store: FileSecretStore,
}

/// Process-wide mutex around every read-modify-write of `credentials.json`.
/// Without it, two concurrent commands (two windows, or a mutation racing an
/// apply) interleave their load/save and the second save silently drops the
/// first's record. Mutations also take the same cross-process ApplyLock as configuration changes.
static VAULT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn lock_vault() -> std::sync::MutexGuard<'static, ()> {
    VAULT_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl CredentialVault {
    /// `~/.claude-config-manager/` (space-free, like the helper paths).
    pub fn for_home(home: &Path) -> Self {
        Self {
            root: home.join(".claude-config-manager"),
            store: FileSecretStore::for_home(home),
        }
    }

    fn secrets_dir(&self) -> PathBuf {
        self.root.join("secrets")
    }

    fn index_path(&self) -> PathBuf {
        index_path(self.root.parent().unwrap_or(&self.root))
    }

    /// Stems of legacy secret files. Listing only — no file is opened.
    fn legacy_stems(&self) -> Vec<String> {
        let Ok(entries) = fs::read_dir(self.secrets_dir()) else {
            return Vec::new();
        };
        entries
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                name.strip_suffix(".secret").map(|s| s.to_string())
            })
            .collect()
    }

    /// Read the index, migrating legacy keys on the fly (in memory only; it is
    /// persisted by the next mutation so a read never writes).
    pub fn load(&self) -> Result<CredentialIndex, VaultError> {
        let path = self.index_path();
        let parsed = match crate::config_reader::read_settings(&path) {
            Ok(value) => CredentialIndex::parse(&value.to_string())?,
            Err(crate::config_reader::ConfigError::Missing) => CredentialIndex::default(),
            Err(e) => return Err(VaultError::Io(e.to_string())),
        };
        let legacy_stems: Vec<String> = self
            .legacy_stems()
            .into_iter()
            .filter(|stem| stem != "proxy-helper" && stem != "proxy")
            .filter(|stem| parsed.get(stem).is_none())
            .filter(|stem| {
                crate::presets::load_builtin_presets()
                    .iter()
                    .any(|p| &p.id == stem)
                    || stem == "custom"
                    || stem.strip_prefix("custom-").is_some_and(|suffix| {
                        !suffix.is_empty()
                            && suffix.len() <= 16
                            && suffix.chars().all(|ch| ch.is_ascii_hexdigit())
                    })
            })
            .collect();
        Ok(migrate_legacy(&parsed, &legacy_stems, &now_rfc3339()))
    }

    /// Persist an index: validate, write 0600 through a temp file, fsync, rename.
    pub fn save(&self, index: &CredentialIndex) -> Result<(), VaultError> {
        index.validate()?;
        let doc = serde_json::to_value(index).map_err(|e| VaultError::Io(e.to_string()))?;
        crate::config_writer::write_settings_atomic(&self.index_path(), &doc, None)
            .map_err(|e| VaultError::Io(e.to_string()))
    }

    pub fn list(&self, provider_id: &str) -> Result<Vec<CredentialRecord>, VaultError> {
        Ok(self
            .load()?
            .for_provider(provider_id)
            .into_iter()
            .cloned()
            .collect())
    }

    /// True when any credential (indexed or legacy) exists for the provider.
    pub fn has_any(&self, provider_id: &str) -> Result<bool, VaultError> {
        Ok(self.resolve(provider_id, None)?.is_some())
    }

    /// Select a usable provider-owned key. All credential consumers use this
    /// path so a legacy file cannot silently override indexed/disabled keys.
    pub fn resolve(
        &self,
        provider_id: &str,
        credential_id: Option<&str>,
    ) -> Result<Option<String>, VaultError> {
        let list = self.list(provider_id)?;
        let record = match credential_id {
            Some(id) => list.iter().find(|c| c.id == id && c.enabled),
            None => list.iter().find(|c| c.enabled),
        };
        match record {
            Some(record) => Ok(self
                .secret(&record.id)?
                .filter(|secret| !secret.trim().is_empty())),
            None => Ok(None),
        }
    }

    /// Next free `<provider>-<n>` id.
    fn next_id(&self, index: &CredentialIndex, provider_id: &str) -> Result<String, VaultError> {
        for n in 1..=999 {
            let id = format!("{provider_id}-{n}");
            if index.get(&id).is_none() {
                validate_credential_id(&id)?;
                return Ok(id);
            }
        }
        Err(VaultError::Invalid(
            "too many credentials for this provider".into(),
        ))
    }

    fn check_transaction(
        &self,
        transaction: &crate::transaction_journal::ManagedTransaction,
    ) -> Result<(), VaultError> {
        let backups = crate::config_backup::default_backup_dir(self.root.parent().unwrap());
        if transaction.lock().path() != backups.parent().unwrap().join("apply.lock") {
            return Err(VaultError::Invalid("wrong transaction scope".into()));
        }
        Ok(())
    }
    fn finish<T>(
        transaction: crate::transaction_journal::ManagedTransaction,
        result: Result<T, VaultError>,
    ) -> Result<T, VaultError> {
        match result {
            Ok(value) => {
                transaction.commit().map_err(VaultError::Io)?;
                Ok(value)
            }
            Err(error) => match transaction.rollback() {
                Ok(()) => Err(error),
                Err(_) => Err(VaultError::Io(format!(
                    "{error}; recovery rollback failed. Review the interrupted-operation recovery before retrying."
                ))),
            },
        }
    }
    /// Add or re-enable an existing identical key. Repeated failed setup retries
    /// never create duplicate secret files or consume all index slots.
    pub fn add(
        &self,
        provider_id: &str,
        label: &str,
        secret: &str,
    ) -> Result<CredentialRecord, VaultError> {
        let _guard = lock_vault();
        let mut transaction = crate::transaction_journal::ManagedTransaction::begin(
            self.root.parent().unwrap(),
            None,
        )
        .map_err(VaultError::Io)?;
        let result = self.add_locked(provider_id, label, secret, &mut transaction);
        Self::finish(transaction, result)
    }
    pub(crate) fn add_locked(
        &self,
        provider_id: &str,
        label: &str,
        secret: &str,
        transaction: &mut crate::transaction_journal::ManagedTransaction,
    ) -> Result<CredentialRecord, VaultError> {
        self.check_transaction(transaction)?;
        crate::credential_handoff::validate_provider_id(provider_id)
            .map_err(|_| VaultError::Invalid("invalid provider id".into()))?;
        if secret.trim().is_empty() {
            return Err(VaultError::Invalid("credential is empty".into()));
        }
        let mut index = self.load()?;
        for record in index.for_provider(provider_id) {
            if self.secret(&record.id)?.as_deref() == Some(secret) {
                let mut existing = record.clone();
                existing.enabled = true;
                if let Some(slot) = index.credentials.iter_mut().find(|r| r.id == existing.id) {
                    slot.enabled = true;
                }
                self.save(&index)?;
                return Ok(existing);
            }
        }
        let id = self.next_id(&index, provider_id)?;
        let record = CredentialRecord {
            id: id.clone(),
            provider_id: provider_id.into(),
            label: label.trim().into(),
            last4: last4_of(secret),
            priority: index.for_provider(provider_id).len() as u32,
            enabled: true,
            created_at: now_rfc3339(),
        };
        index.insert(record.clone())?;
        transaction
            .track(&self.secrets_dir().join(format!("{id}.secret")))
            .map_err(VaultError::Io)?;
        CredentialService::new(&self.store)
            .set_credential(&id, secret)
            .map_err(|e| VaultError::Secret(e.to_string()))?;
        self.save(&index)?;
        Ok(record)
    }
    /// Delete only an indexed key. An arbitrary unknown account must never
    /// delete an internal proxy token or unrelated orphan file.
    pub fn remove(&self, credential_id: &str) -> Result<bool, VaultError> {
        let _guard = lock_vault();
        let transaction = crate::transaction_journal::ManagedTransaction::begin(
            self.root.parent().unwrap(),
            None,
        )
        .map_err(VaultError::Io)?;
        let result = (|| {
            validate_credential_id(credential_id)?;
            let mut index = self.load()?;
            if !index.remove(credential_id) {
                return Ok(false);
            }
            self.save(&index)?;
            CredentialService::new(&self.store)
                .delete_credential(credential_id)
                .map_err(|e| VaultError::Secret(e.to_string()))?;
            Ok(true)
        })();
        Self::finish(transaction, result)
    }
    pub fn set_enabled(&self, credential_id: &str, enabled: bool) -> Result<bool, VaultError> {
        let _guard = lock_vault();
        let transaction = crate::transaction_journal::ManagedTransaction::begin(
            self.root.parent().unwrap(),
            None,
        )
        .map_err(VaultError::Io)?;
        let result = (|| {
            validate_credential_id(credential_id)?;
            let mut index = self.load()?;
            let Some(slot) = index.credentials.iter_mut().find(|c| c.id == credential_id) else {
                return Ok(false);
            };
            slot.enabled = enabled;
            self.save(&index)?;
            Ok(true)
        })();
        Self::finish(transaction, result)
    }
    pub fn promote(&self, credential_id: &str) -> Result<bool, VaultError> {
        let _guard = lock_vault();
        let mut transaction = crate::transaction_journal::ManagedTransaction::begin(
            self.root.parent().unwrap(),
            None,
        )
        .map_err(VaultError::Io)?;
        let result = self.promote_locked(credential_id, &mut transaction);
        Self::finish(transaction, result)
    }
    pub(crate) fn promote_locked(
        &self,
        credential_id: &str,
        transaction: &mut crate::transaction_journal::ManagedTransaction,
    ) -> Result<bool, VaultError> {
        self.check_transaction(transaction)?;
        validate_credential_id(credential_id)?;
        let mut index = self.load()?;
        if !index.promote(credential_id) {
            return Ok(false);
        }
        self.save(&index)?;
        Ok(true)
    }

    /// The secret for a credential id, never for an arbitrary file name.
    ///
    /// A migrated `<provider>-legacy` record aliases the pre-V2 file
    /// `secrets/<provider>.secret`: the upgrade copies and renames nothing, so
    /// the key keeps living in the exact file Claude Code already reads.
    pub fn secret(&self, credential_id: &str) -> Result<Option<String>, VaultError> {
        validate_credential_id(credential_id)?;
        let index = self.load()?;
        let Some(record) = index.get(credential_id) else {
            return Ok(None);
        };
        let legacy_alias = crate::credential_migration::legacy_credential_id(&record.provider_id);
        let account = if legacy_alias == credential_id {
            record.provider_id.clone()
        } else {
            credential_id.to_string()
        };
        CredentialService::new(&self.store)
            .get_credential(&account)
            .map_err(|e| VaultError::Secret(e.to_string()))
    }
}

/// RFC 3339 UTC timestamp without pulling a date crate in. Used for ordering
/// only, so a second-resolution value is enough.
fn now_rfc3339() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Days since epoch → civil date (Howard Hinnant's algorithm).
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static N: AtomicU32 = AtomicU32::new(0);

    fn temp_home(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ccm-vault-{}-{}-{name}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp home");
        dir
    }

    fn write_legacy(home: &Path, provider: &str, secret: &str) {
        let dir = home.join(".claude-config-manager").join("secrets");
        fs::create_dir_all(&dir).expect("secrets dir");
        let p = dir.join(format!("{provider}.secret"));
        let mut f = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(&p)
            .expect("legacy secret");
        f.write_all(secret.as_bytes()).expect("write");
    }

    #[test]
    fn adds_lists_and_resolves_multiple_credentials_per_provider() {
        let home = temp_home("multi");
        let vault = CredentialVault::for_home(&home);
        let first = vault
            .add("openrouter", "work", "sk-or-v1-aaaa1111")
            .expect("add 1");
        let second = vault
            .add("openrouter", "free", "sk-or-v1-bbbb2222")
            .expect("add 2");
        assert_eq!(first.id, "openrouter-1");
        assert_eq!(second.id, "openrouter-2");
        assert_eq!(first.last4, "1111");
        assert_eq!(second.priority, 1);

        let list = vault.list("openrouter").expect("list");
        assert_eq!(list.len(), 2);
        // Order is priority, so the first added is the first tried.
        assert_eq!(list[0].id, "openrouter-1");
        assert_eq!(
            vault.secret("openrouter-2").expect("resolve"),
            Some("sk-or-v1-bbbb2222".into())
        );
        // The index never carries key material.
        let raw = fs::read_to_string(home.join(".claude-config-manager/credentials.json"))
            .expect("index file");
        assert!(!raw.contains("bbbb2222"));
        // Index file is owner-only.
        let mode = fs::metadata(home.join(".claude-config-manager/credentials.json"))
            .expect("meta")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn persists_across_instances_and_keeps_chain_order() {
        let home = temp_home("persist");
        {
            let vault = CredentialVault::for_home(&home);
            vault
                .add("openrouter", "a", "sk-or-v1-aaaa")
                .expect("add a");
            vault
                .add("openrouter", "b", "sk-or-v1-bbbb")
                .expect("add b");
        }
        let vault = CredentialVault::for_home(&home);
        assert!(vault.promote("openrouter-2").expect("promote"));
        let ids: Vec<String> = vault
            .list("openrouter")
            .expect("list")
            .into_iter()
            .map(|c| c.id)
            .collect();
        assert_eq!(ids, ["openrouter-2", "openrouter-1"]);
    }

    #[test]
    fn remove_deletes_the_secret_and_is_idempotent() {
        let home = temp_home("remove");
        let vault = CredentialVault::for_home(&home);
        vault.add("openrouter", "a", "sk-or-v1-aaaa").expect("add");
        let secret_path = home.join(".claude-config-manager/secrets/openrouter-1.secret");
        assert!(secret_path.exists());
        assert!(vault.remove("openrouter-1").expect("remove"));
        assert!(!secret_path.exists());
        assert!(vault.list("openrouter").expect("list").is_empty());
        assert!(!vault.remove("openrouter-1").expect("second remove"));
    }

    #[test]
    fn disabling_hides_a_credential_from_the_active_chain_but_keeps_it() {
        let home = temp_home("disable");
        let vault = CredentialVault::for_home(&home);
        vault.add("openrouter", "a", "sk-or-v1-aaaa").expect("add");
        assert!(vault.set_enabled("openrouter-1", false).expect("disable"));
        let index = vault.load().expect("load");
        assert!(index.active_for_provider("openrouter").is_empty());
        assert_eq!(index.for_provider("openrouter").len(), 1);
        assert!(vault.secret("openrouter-1").expect("secret").is_some());
        assert!(!vault.set_enabled("missing-1", false).expect("unknown"));
    }

    #[test]
    fn legacy_secret_is_seen_and_still_resolvable_after_an_upgrade() {
        let home = temp_home("legacy");
        write_legacy(&home, "openrouter", "sk-or-v1-legacy");
        let vault = CredentialVault::for_home(&home);
        let list = vault.list("openrouter").expect("list");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "openrouter-legacy");
        assert_eq!(
            vault.secret("openrouter-legacy").expect("resolve"),
            Some("sk-or-v1-legacy".into())
        );
        // Adding a new key leaves the legacy file untouched and orders the new
        // key after it.
        vault.add("openrouter", "new", "sk-or-v1-new").expect("add");
        let ids: Vec<String> = vault
            .list("openrouter")
            .expect("list")
            .into_iter()
            .map(|c| c.id)
            .collect();
        assert_eq!(ids, ["openrouter-legacy", "openrouter-1"]);
    }

    #[test]
    fn refuses_unsafe_ids_and_blank_credentials() {
        let home = temp_home("reject");
        let vault = CredentialVault::for_home(&home);
        assert!(vault.add("openrouter", "empty", "   ").is_err());
        assert!(vault.secret("../escape").is_err());
        assert!(vault.remove("Bad/Id").is_err());
        // Nothing was written for the rejected attempts.
        assert!(
            !home
                .join(".claude-config-manager/credentials.json")
                .exists()
        );
    }

    #[test]
    fn timestamp_is_rfc3339_shaped() {
        let now = now_rfc3339();
        assert_eq!(now.len(), 20, "{now}");
        assert!(now.ends_with('Z'));
        assert_eq!(now.as_bytes()[4], b'-');
    }
}
