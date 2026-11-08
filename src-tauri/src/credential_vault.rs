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
use std::io::Write;
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
        let parsed = match fs::read_to_string(&path) {
            Ok(raw) => CredentialIndex::parse(&raw)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => CredentialIndex::default(),
            Err(e) => return Err(VaultError::Io(e.to_string())),
        };
        Ok(migrate_legacy(
            &parsed,
            &self.legacy_stems(),
            &now_rfc3339(),
        ))
    }

    /// Persist an index: validate, write 0600 through a temp file, fsync, rename.
    pub fn save(&self, index: &CredentialIndex) -> Result<(), VaultError> {
        let json = index.to_json()?;
        fs::create_dir_all(&self.root).map_err(|e| VaultError::Io(e.to_string()))?;
        let target = self.index_path();
        let tmp = target.with_extension("json.tmp");
        {
            let mut f = fs::OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .mode(0o600)
                .open(&tmp)
                .map_err(|e| VaultError::Io(e.to_string()))?;
            f.write_all(json.as_bytes())
                .map_err(|e| VaultError::Io(e.to_string()))?;
            f.sync_all().map_err(|e| VaultError::Io(e.to_string()))?;
        }
        fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600))
            .map_err(|e| VaultError::Io(e.to_string()))?;
        fs::rename(&tmp, &target).map_err(|e| VaultError::Io(e.to_string()))?;
        Ok(())
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
        Ok(!self.load()?.for_provider(provider_id).is_empty())
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

    /// Add a credential. The secret is written first; if the index write then
    /// fails the secret is deleted again, so no orphan key is left behind.
    pub fn add(
        &self,
        provider_id: &str,
        label: &str,
        secret: &str,
    ) -> Result<CredentialRecord, VaultError> {
        crate::credential_handoff::validate_provider_id(provider_id)
            .map_err(|_| VaultError::Invalid("invalid provider id".into()))?;
        if secret.trim().is_empty() {
            return Err(VaultError::Invalid("credential is empty".into()));
        }
        let mut index = self.load()?;
        let id = self.next_id(&index, provider_id)?;
        let creds = CredentialService::new(&self.store);
        creds
            .set_credential(&id, secret)
            .map_err(|e| VaultError::Secret(e.to_string()))?;

        let priority = index.for_provider(provider_id).len() as u32;
        let record = CredentialRecord {
            id: id.clone(),
            provider_id: provider_id.to_string(),
            label: label.trim().to_string(),
            last4: last4_of(secret),
            priority,
            enabled: true,
            created_at: now_rfc3339(),
        };
        if let Err(e) = index.insert(record.clone()) {
            let _ = creds.delete_credential(&id);
            return Err(e.into());
        }
        if let Err(e) = self.save(&index) {
            let _ = creds.delete_credential(&id);
            return Err(e);
        }
        Ok(record)
    }

    /// Delete a credential and its secret. Idempotent: `false` when unknown.
    pub fn remove(&self, credential_id: &str) -> Result<bool, VaultError> {
        validate_credential_id(credential_id)?;
        let mut index = self.load()?;
        if !index.remove(credential_id) {
            return Ok(false);
        }
        CredentialService::new(&self.store)
            .delete_credential(credential_id)
            .map_err(|e| VaultError::Secret(e.to_string()))?;
        self.save(&index)?;
        Ok(true)
    }

    pub fn set_enabled(&self, credential_id: &str, enabled: bool) -> Result<bool, VaultError> {
        validate_credential_id(credential_id)?;
        let mut index = self.load()?;
        let Some(slot) = index.credentials.iter_mut().find(|c| c.id == credential_id) else {
            return Ok(false);
        };
        slot.enabled = enabled;
        self.save(&index)?;
        Ok(true)
    }

    /// Move a credential to the front of its provider's chain.
    pub fn promote(&self, credential_id: &str) -> Result<bool, VaultError> {
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
