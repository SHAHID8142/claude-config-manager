//! Store that tries the Security framework first and the `security` CLI second.
//!
//! Reported failure: the bundled app's SecItemAdd was refused by macOS
//! ("keychain access refused by the OS") and Save & Apply failed even though the
//! login keychain was usable — the same credentials could be read by the helper
//! script, which shells out to `security`. Rather than guess at the cause of a
//! refusal we cannot reproduce, the app now degrades instead of failing: if the
//! framework path refuses, the CLI path is attempted for the same service and the
//! operation succeeds.
//!
//! Ordering matters for security: the framework path is preferred because it
//! never puts the secret in another process. The fallback is used only after a
//! platform refusal, and it is the exact mechanism Claude Code already relies on
//! at request time.

use crate::secret_store::{SecretError, SecretStore};

pub struct FallbackStore {
    primary: Box<dyn SecretStore>,
    fallback: Box<dyn SecretStore>,
}

impl FallbackStore {
    pub fn new(primary: Box<dyn SecretStore>, fallback: Box<dyn SecretStore>) -> Self {
        Self { primary, fallback }
    }
}

impl std::fmt::Debug for FallbackStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FallbackStore")
    }
}

/// Only a *platform* failure justifies retrying: a genuinely missing item or an
/// invalid identifier means the fallback would fail the same way or make things
/// worse.
fn should_fallback(err: &SecretError) -> bool {
    matches!(err, SecretError::Platform(_))
}

impl SecretStore for FallbackStore {
    fn set_secret(&self, service: &str, account: &str, secret: &str) -> Result<(), SecretError> {
        match self.primary.set_secret(service, account, secret) {
            Ok(()) => Ok(()),
            Err(e) if should_fallback(&e) => {
                self.fallback
                    .set_secret(service, account, secret)
                    .map_err(|second| {
                        // Report both causes: the first explains the configuration,
                        // the second is what the user is stuck with.
                        SecretError::Platform(format!("{e}; fallback also failed: {second}"))
                    })
            }
            Err(e) => Err(e),
        }
    }

    fn get_secret(&self, service: &str, account: &str) -> Result<Option<String>, SecretError> {
        match self.primary.get_secret(service, account) {
            // A missing item is authoritative: the fallback reads the same item.
            Ok(result) => Ok(result),
            Err(e) if should_fallback(&e) => self.fallback.get_secret(service, account),
            Err(e) => Err(e),
        }
    }

    fn delete_secret(&self, service: &str, account: &str) -> Result<(), SecretError> {
        match self.primary.delete_secret(service, account) {
            Ok(()) => Ok(()),
            Err(e) if should_fallback(&e) => self.fallback.delete_secret(service, account),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Recorder {
        sets: Mutex<Vec<(String, String)>>,
    }

    struct AlwaysFails;
    impl SecretStore for AlwaysFails {
        fn set_secret(&self, _: &str, _: &str, _: &str) -> Result<(), SecretError> {
            Err(SecretError::Platform(
                "keychain access failed (-25293)".into(),
            ))
        }
        fn get_secret(&self, _: &str, _: &str) -> Result<Option<String>, SecretError> {
            Err(SecretError::Platform(
                "keychain access failed (-25293)".into(),
            ))
        }
        fn delete_secret(&self, _: &str, _: &str) -> Result<(), SecretError> {
            Err(SecretError::Platform(
                "keychain access failed (-25293)".into(),
            ))
        }
    }

    impl SecretStore for Recorder {
        fn set_secret(&self, service: &str, account: &str, _: &str) -> Result<(), SecretError> {
            self.sets
                .lock()
                .expect("lock")
                .push((service.to_string(), account.to_string()));
            Ok(())
        }
        fn get_secret(&self, _: &str, _: &str) -> Result<Option<String>, SecretError> {
            Ok(Some("value".into()))
        }
        fn delete_secret(&self, _: &str, _: &str) -> Result<(), SecretError> {
            Ok(())
        }
    }

    struct MissingItem;
    impl SecretStore for MissingItem {
        fn set_secret(&self, _: &str, _: &str, _: &str) -> Result<(), SecretError> {
            Err(SecretError::ItemNotFound)
        }
        fn get_secret(&self, _: &str, _: &str) -> Result<Option<String>, SecretError> {
            Ok(None)
        }
        fn delete_secret(&self, _: &str, _: &str) -> Result<(), SecretError> {
            Err(SecretError::ItemNotFound)
        }
    }

    #[test]
    fn a_platform_refusal_falls_back_and_the_save_succeeds() {
        let rec = std::sync::Arc::new(Recorder::default());
        struct Shared(std::sync::Arc<Recorder>);
        impl SecretStore for Shared {
            fn set_secret(&self, s: &str, a: &str, v: &str) -> Result<(), SecretError> {
                self.0.set_secret(s, a, v)
            }
            fn get_secret(&self, s: &str, a: &str) -> Result<Option<String>, SecretError> {
                self.0.get_secret(s, a)
            }
            fn delete_secret(&self, s: &str, a: &str) -> Result<(), SecretError> {
                self.0.delete_secret(s, a)
            }
        }
        let store = FallbackStore::new(Box::new(AlwaysFails), Box::new(Shared(rec.clone())));
        store
            .set_secret("com.claude-config-manager.deepseek", "deepseek", "k")
            .expect("fallback must save the credential");
        assert_eq!(rec.sets.lock().expect("lock").len(), 1);
    }

    #[test]
    fn a_missing_item_does_not_trigger_the_fallback() {
        let store = FallbackStore::new(Box::new(MissingItem), Box::new(Recorder::default()));
        assert_eq!(
            store.get_secret("s", "a").expect("ok"),
            None,
            "ItemNotFound is authoritative — the fallback reads the same item"
        );
    }

    #[test]
    fn both_failing_reports_the_first_cause_too() {
        let store = FallbackStore::new(Box::new(AlwaysFails), Box::new(AlwaysFails));
        let err = store.set_secret("s", "a", "k").expect_err("must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("-25293"),
            "first cause must be preserved: {msg}"
        );
        assert!(msg.contains("fallback also failed"));
    }
}
