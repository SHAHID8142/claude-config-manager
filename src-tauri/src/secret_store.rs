//! Credential vault interface (T21): storage-agnostic `SecretStore` trait.
//!
//! Contract (docs/architecture/MODULE-BOUNDARIES.md):
//! - Secrets never appear in error messages or logs; errors carry only
//!   structural context (account name is an identifier, not a secret).
//! - The macOS Keychain-backed store is T22; the mock here serves tests and
//!   non-macOS builds until platform stores land.
//! - Deleting a missing secret is idempotent (Ok) — delete is a promise the
//!   secret is gone afterwards, not a report about prior state.

use std::collections::HashMap;
use std::sync::Mutex;

/// Errors surfaced by vault operations. Never embeds secret material.
#[derive(Debug, Clone, PartialEq)]
pub enum SecretError {
    /// Underlying OS vault refused the operation (message pre-redacted by
    /// the platform store; the trait contract forbids echoing secret data).
    Platform(String),
    /// No secret exists under this account.
    ItemNotFound,
    /// Account/service identifier failed validation (empty or oversized).
    InvalidIdentifier,
}

impl std::fmt::Display for SecretError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecretError::Platform(m) => write!(f, "credential vault error: {m}"),
            SecretError::ItemNotFound => write!(f, "no stored credential for this account"),
            SecretError::InvalidIdentifier => write!(f, "invalid credential identifier"),
        }
    }
}

impl std::error::Error for SecretError {}

/// Storage-agnostic credential vault. All methods are blocking and must be
/// called from a context where brief blocking is acceptable (Tauri commands).
pub trait SecretStore: Send + Sync {
    /// Store or replace the secret for `account` under `service`.
    fn set_secret(&self, service: &str, account: &str, secret: &str) -> Result<(), SecretError>;
    /// Fetch the secret, if present.
    fn get_secret(&self, service: &str, account: &str) -> Result<Option<String>, SecretError>;
    /// Remove the secret. Idempotent.
    fn delete_secret(&self, service: &str, account: &str) -> Result<(), SecretError>;
}

/// Validate identifier shape shared by all stores.
fn validate_identifier(service: &str, account: &str) -> Result<(), SecretError> {
    for (kind, v) in [("service", service), ("account", account)] {
        if v.trim().is_empty() || v.len() > 256 || v.contains('\0') {
            return Err(SecretError::InvalidIdentifier);
        }
        let _ = kind;
    }
    Ok(())
}

/// In-memory vault for tests and mock transports. Thread-safe.
#[derive(Default)]
pub struct MockSecretStore {
    items: Mutex<HashMap<(String, String), String>>,
}

impl MockSecretStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Test helper: number of stored items.
    pub fn len(&self) -> usize {
        self.items.lock().expect("mock store lock").len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl SecretStore for MockSecretStore {
    fn set_secret(&self, service: &str, account: &str, secret: &str) -> Result<(), SecretError> {
        validate_identifier(service, account)?;
        if secret.is_empty() {
            // An empty secret is never stored; callers clear via delete.
            return Err(SecretError::InvalidIdentifier);
        }
        self.items.lock().expect("mock store lock").insert(
            (service.to_string(), account.to_string()),
            secret.to_string(),
        );
        Ok(())
    }

    fn get_secret(&self, service: &str, account: &str) -> Result<Option<String>, SecretError> {
        validate_identifier(service, account)?;
        Ok(self
            .items
            .lock()
            .expect("mock store lock")
            .get(&(service.to_string(), account.to_string()))
            .cloned())
    }

    fn delete_secret(&self, service: &str, account: &str) -> Result<(), SecretError> {
        validate_identifier(service, account)?;
        self.items
            .lock()
            .expect("mock store lock")
            .remove(&(service.to_string(), account.to_string()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SVC: &str = "com.claude-config-manager.test-provider";

    #[test]
    fn roundtrip_set_get_delete() {
        let store = MockSecretStore::new();
        store
            .set_secret(SVC, "anthropic", "sk-test-FAKE-0001")
            .expect("set");
        assert_eq!(
            store.get_secret(SVC, "anthropic").expect("get"),
            Some("sk-test-FAKE-0001".to_string())
        );
        store.delete_secret(SVC, "anthropic").expect("delete");
        assert_eq!(store.get_secret(SVC, "anthropic").expect("get"), None);
    }

    #[test]
    fn set_replaces_existing_secret() {
        let store = MockSecretStore::new();
        store
            .set_secret(SVC, "anthropic", "sk-test-OLD")
            .expect("set old");
        store
            .set_secret(SVC, "anthropic", "sk-test-NEW")
            .expect("replace");
        assert_eq!(
            store.get_secret(SVC, "anthropic").expect("get"),
            Some("sk-test-NEW".to_string())
        );
        assert_eq!(store.len(), 1, "replace must not duplicate items");
    }

    #[test]
    fn accounts_are_isolated() {
        let store = MockSecretStore::new();
        store
            .set_secret(SVC, "anthropic", "sk-test-A")
            .expect("set a");
        store
            .set_secret(SVC, "other-provider", "sk-test-B")
            .expect("set b");
        assert_eq!(
            store.get_secret(SVC, "anthropic").expect("get"),
            Some("sk-test-A".to_string())
        );
        assert_eq!(
            store.get_secret(SVC, "other-provider").expect("get"),
            Some("sk-test-B".to_string())
        );
    }

    #[test]
    fn delete_missing_is_idempotent() {
        let store = MockSecretStore::new();
        store
            .delete_secret(SVC, "never-stored")
            .expect("idempotent delete");
    }

    #[test]
    fn empty_identifiers_rejected() {
        let store = MockSecretStore::new();
        assert!(matches!(
            store.set_secret(SVC, "", "x"),
            Err(SecretError::InvalidIdentifier)
        ));
        assert!(matches!(
            store.set_secret("", "a", "x"),
            Err(SecretError::InvalidIdentifier)
        ));
        let long = "a".repeat(257);
        assert!(matches!(
            store.set_secret(SVC, &long, "x"),
            Err(SecretError::InvalidIdentifier)
        ));
    }

    #[test]
    fn empty_secret_rejected() {
        let store = MockSecretStore::new();
        assert!(matches!(
            store.set_secret(SVC, "anthropic", ""),
            Err(SecretError::InvalidIdentifier)
        ));
    }

    #[test]
    fn error_display_carries_no_secret_material() {
        let err = SecretError::Platform("errSecItemNotFound".to_string());
        let rendered = err.to_string();
        assert!(!rendered.contains("sk-"));
        assert!(
            matches!(SecretError::ItemNotFound.to_string(), s if s.contains("no stored credential"))
        );
    }

    #[test]
    fn store_is_thread_safe() {
        let store = std::sync::Arc::new(MockSecretStore::new());
        let handles: Vec<_> = (0..8)
            .map(|i| {
                let s = store.clone();
                std::thread::spawn(move || {
                    s.set_secret(SVC, &format!("acct-{i}"), "sk-test-x")
                        .expect("set");
                })
            })
            .collect();
        for h in handles {
            h.join().expect("thread");
        }
        assert_eq!(store.len(), 8);
    }
}
