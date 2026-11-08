//! macOS Keychain-backed `SecretStore` (T22).
//!
//! Uses the `keyring` crate (v4) against the login keychain. Windows
//! Credential Manager / Linux secret-service backends come from the same
//! crate — the trait boundary from T21 keeps platform swaps local.
//!
//! Error contract: platform errors are reduced to short, pre-redacted
//! descriptions (keyring's own Display carries error *codes*, never secret
//! material; we do not include user-supplied values in any error string).

use crate::secret_store::{SecretError, SecretStore};

/// Keychain-backed store. Construction is cheap; entries are opened per call.
#[derive(Debug, Default)]
pub struct KeychainStore;

impl KeychainStore {
    pub fn new() -> Self {
        Self
    }
}

/// Reduce a keyring error to a redaction-safe platform message.
fn map_keyring_error(err: &keyring::Error) -> SecretError {
    match err {
        // Distinguishable "missing" outcomes are handled by callers via
        // Option/Ok; anything reaching here is unexpected.
        keyring::Error::NoEntry => SecretError::ItemNotFound,
        keyring::Error::Ambiguous(_) => {
            SecretError::Platform("ambiguous credential entry".to_string())
        }
        keyring::Error::BadEncoding(_) => {
            SecretError::Platform("credential data unreadable".to_string())
        }
        keyring::Error::TooLong(_, _) => SecretError::InvalidIdentifier,
        keyring::Error::Invalid(_attr, _reason) => {
            SecretError::Platform("credential identifier rejected".to_string())
        }
        // Keep the OS-level cause (an OSStatus or short platform string — never
        // secret material) so a refusal is diagnosable instead of opaque. The
        // first shipped build reported only "refused by the OS", which made the
        // real cause impossible to identify from the UI.
        // The detail is kept for diagnosis but passed through the secret-shaped
        // stripper first, so no credential-like token can ride along.
        keyring::Error::PlatformFailure(inner) => SecretError::Platform(format!(
            "keychain access failed ({})",
            crate::redact::strip_secret_shaped(&inner.to_string())
        )),
        keyring::Error::NoStorageAccess(inner) => SecretError::Platform(format!(
            "keychain storage unavailable ({})",
            crate::redact::strip_secret_shaped(&inner.to_string())
        )),
        other => SecretError::Platform(format!("credential vault error ({other})")),
    }
}

fn entry(service: &str, account: &str) -> Result<keyring::Entry, SecretError> {
    keyring::Entry::new(service, account).map_err(|e| map_keyring_error(&e))
}

impl SecretStore for KeychainStore {
    fn set_secret(&self, service: &str, account: &str, secret: &str) -> Result<(), SecretError> {
        let e = entry(service, account)?;
        e.set_password(secret)
            .map_err(|err| map_keyring_error(&err))
    }

    fn get_secret(&self, service: &str, account: &str) -> Result<Option<String>, SecretError> {
        let e = entry(service, account)?;
        match e.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(err) => Err(map_keyring_error(&err)),
        }
    }

    fn delete_secret(&self, service: &str, account: &str) -> Result<(), SecretError> {
        let e = entry(service, account)?;
        match e.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()), // idempotent per T21 contract
            Err(err) => Err(map_keyring_error(&err)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_item_maps_to_item_not_found() {
        assert_eq!(
            map_keyring_error(&keyring::Error::NoEntry),
            SecretError::ItemNotFound
        );
    }

    #[test]
    fn platform_refusals_keep_the_os_cause_but_drop_secrets() {
        // Policy (G11): the OSStatus is now KEPT so a refusal is diagnosable —
        // the previous opaque "refused by the OS" made the shipped failure
        // impossible to investigate. Credential-shaped tokens are still stripped.
        let err = map_keyring_error(&keyring::Error::PlatformFailure(
            "OSStatus error -25293 sk-ant-api03-LEAKEDLEAKEDLEAKED".into(),
        ));
        match err {
            SecretError::Platform(msg) => {
                assert!(msg.contains("-25293"), "cause must survive: {msg}");
                assert!(msg.contains("keychain access failed"));
                assert!(!msg.contains("sk-ant-api03-LEAKED"), "secret leaked: {msg}");
            }
            other => panic!("expected Platform error, got {other:?}"),
        }
    }

    #[test]
    fn oversized_maps_to_invalid_identifier() {
        assert_eq!(
            map_keyring_error(&keyring::Error::TooLong("attr".into(), 32)),
            SecretError::InvalidIdentifier
        );
    }

    /// Real login-keychain round trip (macOS only). Creates a uniquely named
    /// item under the `ccm-selftest-` prefix and deletes it — per the T07
    /// test strategy, this is the only test touching the real keychain.
    #[cfg(target_os = "macos")]
    #[test]
    fn keychain_round_trip_real() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let uniq = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let service = "com.claude-config-manager.selftest";
        let account = format!("ccm-selftest-{uniq}");
        let store = KeychainStore::new();
        let fake = format!("sk-test-SELFTEST-{uniq}");

        store
            .set_secret(service, &account, &fake)
            .expect("keychain set");
        let got = store.get_secret(service, &account).expect("keychain get");
        assert_eq!(got.as_deref(), Some(fake.as_str()));

        store
            .delete_secret(service, &account)
            .expect("keychain delete");
        assert_eq!(
            store
                .get_secret(service, &account)
                .expect("get after delete"),
            None
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn keychain_delete_missing_is_idempotent_real() {
        let service = "com.claude-config-manager.selftest";
        let account = "ccm-selftest-never-existed";
        let store = KeychainStore::new();
        store.delete_secret(service, account).expect("idempotent");
    }
}
