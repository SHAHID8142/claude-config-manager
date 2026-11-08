//! Credential lifecycle service (T23): store / replace / delete / status for
//! provider credentials on top of any `SecretStore`, with redaction applied
//! to every error surfaced outward (threat T2).

use crate::redact::redact_secrets;
use crate::secret_store::{SecretError, SecretStore};

/// Service prefix for keychain items; full service name is
/// `<prefix>.<provider_id>`.
pub const SERVICE_PREFIX: &str = "com.claude-config-manager";

pub fn service_name(provider_id: &str) -> Result<String, SecretError> {
    if provider_id.trim().is_empty()
        || provider_id.len() > 64
        || !provider_id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(SecretError::InvalidIdentifier);
    }
    Ok(format!("{SERVICE_PREFIX}.{provider_id}"))
}

fn surface(err: SecretError, live_secrets: &[&str]) -> SecretError {
    match err {
        SecretError::Platform(msg) => SecretError::Platform(redact_secrets(&msg, live_secrets)),
        other => other,
    }
}

/// Lifecycle operations. All errors returned outward are redaction-checked.
pub struct CredentialService<'a> {
    store: &'a dyn SecretStore,
}

impl<'a> CredentialService<'a> {
    pub fn new(store: &'a dyn SecretStore) -> Self {
        Self { store }
    }

    /// Store the credential, replacing any existing one for the provider.
    pub fn set_credential(&self, provider_id: &str, secret: &str) -> Result<(), SecretError> {
        let service = service_name(provider_id)?;
        self.store
            .set_secret(&service, provider_id, secret)
            .map_err(|e| surface(e, &[secret]))
    }

    pub fn has_credential(&self, provider_id: &str) -> Result<bool, SecretError> {
        let service = service_name(provider_id)?;
        self.store
            .get_secret(&service, provider_id)
            .map(|v| v.is_some())
            .map_err(|e| surface(e, &[]))
    }

    /// Read the credential back (used by the V2 credential vault, which must
    /// resolve a *specific* credential id rather than a provider default).
    pub fn get_credential(&self, provider_id: &str) -> Result<Option<String>, SecretError> {
        let service = service_name(provider_id)?;
        self.store
            .get_secret(&service, provider_id)
            .map_err(|e| surface(e, &[]))
    }

    /// Delete the credential. Idempotent.
    pub fn delete_credential(&self, provider_id: &str) -> Result<(), SecretError> {
        let service = service_name(provider_id)?;
        self.store
            .delete_secret(&service, provider_id)
            .map_err(|e| surface(e, &[]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secret_store::MockSecretStore;

    #[test]
    fn lifecycle_set_replace_delete_status() {
        let store = MockSecretStore::new();
        let svc = CredentialService::new(&store);

        assert!(!svc.has_credential("anthropic").expect("status"));
        svc.set_credential("anthropic", "sk-ant-api03-FAKE-0001")
            .expect("set");
        assert!(svc.has_credential("anthropic").expect("status"));

        // Replace in place (no duplicate items).
        svc.set_credential("anthropic", "sk-ant-api03-FAKE-0002")
            .expect("replace");
        assert_eq!(
            store
                .get_secret("com.claude-config-manager.anthropic", "anthropic")
                .expect("peek")
                .as_deref(),
            Some("sk-ant-api03-FAKE-0002")
        );

        svc.delete_credential("anthropic").expect("delete");
        assert!(!svc.has_credential("anthropic").expect("status"));
        // Idempotent second delete.
        svc.delete_credential("anthropic").expect("delete again");
    }

    #[test]
    fn service_name_validation() {
        assert!(service_name("anthropic").is_ok());
        assert!(service_name("open-router-2").is_ok());
        assert!(service_name("").is_err());
        assert!(service_name("Open Router").is_err());
        assert!(service_name(&"a".repeat(65)).is_err());
    }

    #[test]
    fn provider_credentials_are_isolated_per_service() {
        let store = MockSecretStore::new();
        let svc = CredentialService::new(&store);
        svc.set_credential("anthropic", "sk-ant-api03-A")
            .expect("a");
        svc.set_credential("openrouter", "sk-or-v1-B").expect("b");
        assert!(svc.has_credential("anthropic").expect("a?"));
        assert!(svc.has_credential("openrouter").expect("b?"));
        svc.delete_credential("anthropic").expect("del a");
        assert!(
            svc.has_credential("openrouter").expect("b?"),
            "other provider untouched"
        );
    }

    #[test]
    fn platform_errors_are_redacted_before_surfacing() {
        use std::collections::HashMap;
        use std::sync::Mutex;

        struct LeakyStore {
            items: Mutex<HashMap<(String, String), String>>,
        }
        impl LeakyStore {
            fn new() -> Self {
                Self {
                    items: Mutex::new(HashMap::new()),
                }
            }
        }
        impl SecretStore for LeakyStore {
            fn set_secret(&self, _s: &str, _a: &str, secret: &str) -> Result<(), SecretError> {
                // Simulate an OS error that echoes the secret (threat T2).
                Err(SecretError::Platform(format!(
                    "errSecParam while writing {secret}"
                )))
            }
            fn get_secret(&self, _s: &str, _a: &str) -> Result<Option<String>, SecretError> {
                Ok(self
                    .items
                    .lock()
                    .unwrap()
                    .get(&(_s.to_string(), _a.to_string()))
                    .cloned())
            }
            fn delete_secret(&self, _s: &str, _a: &str) -> Result<(), SecretError> {
                Ok(())
            }
        }

        let store = LeakyStore::new();
        let svc = CredentialService::new(&store);
        let secret = "sk-ant-api03-LEAKED-VALUE";
        let err = svc
            .set_credential("anthropic", secret)
            .expect_err("must fail");
        match err {
            SecretError::Platform(msg) => {
                assert!(!msg.contains("LEAKED-VALUE"), "secret leaked: {msg}");
                assert!(msg.contains("[REDACTED]"), "expected redaction: {msg}");
            }
            other => panic!("expected Platform error, got {other:?}"),
        }
    }

    #[test]
    fn empty_secret_rejected_by_underlying_store() {
        let store = MockSecretStore::new();
        let svc = CredentialService::new(&store);
        let err = svc
            .set_credential("anthropic", "")
            .expect_err("must reject");
        assert_eq!(err, SecretError::InvalidIdentifier);
    }
}
