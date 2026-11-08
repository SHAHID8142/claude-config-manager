// Standalone keychain probe (T22): verifies set/get/delete against the real
// macOS data-protection keychain. Uses a fresh unique account per run so the
// operation stays within one process instance's ACL (cross-instance access
// to a prior instance's item triggers a macOS authorization prompt — by
// design; never bypass OS approval).
use claude_config_manager_lib::keychain_store::KeychainStore;
use claude_config_manager_lib::secret_store::SecretStore;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    let uniq = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let store = KeychainStore::new();
    let svc = "com.claude-config-manager.selftest";
    let acct = format!("ccm-selftest-probe-{uniq}");
    let fake = format!("sk-test-PROBE-{uniq}");
    println!("set: {:?}", store.set_secret(svc, &acct, &fake));
    println!("get: {:?}", store.get_secret(svc, &acct));
    println!("delete: {:?}", store.delete_secret(svc, &acct));
    println!("get-after-delete: {:?}", store.get_secret(svc, &acct));
}
