//! G11 — prove the fallback store against the REAL login keychain.
//!
//! `#[ignore]`: touches the user's keychain (with a dedicated probe service that
//! is deleted afterwards). Run with:
//!   cargo test --test keychain_fallback -- --ignored --nocapture

use claude_config_manager_lib::fallback_store::FallbackStore;
use claude_config_manager_lib::secret_store::{SecretError, SecretStore};
use claude_config_manager_lib::security_cli_store::SecurityCliStore;

struct RefusingFramework;

impl SecretStore for RefusingFramework {
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

#[test]
#[ignore = "touches the real login keychain (probe item, deleted afterwards)"]
fn fallback_round_trips_through_the_real_keychain() {
    let svc = "com.claude-config-manager.g11probe";
    let acct = "g11probe";
    let store = FallbackStore::new(
        Box::new(RefusingFramework),
        Box::new(SecurityCliStore::new()),
    );

    store
        .set_secret(svc, acct, "sk-ant-FAKE-g11-probe")
        .expect("fallback write");
    assert_eq!(
        store.get_secret(svc, acct).expect("read"),
        Some("sk-ant-FAKE-g11-probe".to_string()),
        "the value written by the fallback must be readable by the same mechanism the helper uses"
    );
    store.delete_secret(svc, acct).expect("delete");
    assert_eq!(
        store.get_secret(svc, acct).expect("read after delete"),
        None
    );
}
