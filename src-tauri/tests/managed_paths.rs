//! Fixture-only regressions for private managed-file writes and path handling.
use claude_config_manager_lib::{
    credential_handoff::{
        default_helpers_dir, generate_helper_script, install_helper_script, uninstall_helper_script,
    },
    file_secret_store::FileSecretStore,
    secret_store::SecretStore,
};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    sync::{
        Arc, Barrier,
        atomic::{AtomicU64, Ordering},
    },
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let home = std::env::temp_dir().join(format!(
            "ccm-managed-paths-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&home).unwrap();
        Self(home)
    }
    fn secrets(&self) -> PathBuf {
        self.0.join(".claude-config-manager/secrets")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
const SERVICE: &str = "com.claude-config-manager.openrouter";
const ACCOUNT: &str = "openrouter";
fn mode(path: &Path) -> u32 {
    fs::metadata(path).unwrap().permissions().mode() & 0o777
}

#[test]
fn thirty_two_same_account_writers_commit_complete_values_without_temp_collisions() {
    let home = Fixture::new();
    let barrier = Arc::new(Barrier::new(32));
    let mut writers = Vec::new();
    for i in 0..32 {
        let store = FileSecretStore::for_home(&home.0);
        let barrier = barrier.clone();
        writers.push(std::thread::spawn(move || {
            barrier.wait();
            store.set_secret(SERVICE, ACCOUNT, &format!("fixture-complete-value-{i}"))
        }));
    }
    for writer in writers {
        writer
            .join()
            .unwrap()
            .expect("every concurrent write succeeds");
    }
    let value = FileSecretStore::for_home(&home.0)
        .get_secret(SERVICE, ACCOUNT)
        .unwrap()
        .unwrap();
    assert!((0..32).any(|i| value == format!("fixture-complete-value-{i}")));
    assert_eq!(mode(&home.secrets().join("openrouter.secret")), 0o600);
    assert_eq!(
        fs::read_dir(home.secrets()).unwrap().count(),
        1,
        "no writer's temp remains"
    );
}

#[test]
fn replacement_does_not_unlink_the_previous_predictable_temp_name() {
    let home = Fixture::new();
    fs::create_dir_all(home.secrets()).unwrap();
    let other_writer = home.secrets().join("openrouter.secret.tmp");
    fs::write(&other_writer, "other writer's in-progress bytes").unwrap();
    FileSecretStore::for_home(&home.0)
        .set_secret(SERVICE, ACCOUNT, "fixture-new-value")
        .unwrap();
    assert_eq!(
        fs::read_to_string(other_writer).unwrap(),
        "other writer's in-progress bytes"
    );
}

#[test]
fn corrupt_empty_or_control_bearing_credentials_are_not_usable_keys() {
    let home = Fixture::new();
    fs::create_dir_all(home.secrets()).unwrap();
    let path = home.secrets().join("openrouter.secret");
    let store = FileSecretStore::for_home(&home.0);
    for invalid in [
        "",
        "\n",
        "\r\n",
        "\n\n",
        " \t",
        "fixture\nembedded",
        "fixture\0embedded",
        "fixture\n\n",
        "fixture\r",
    ] {
        fs::write(&path, invalid).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(
            store.get_secret(SERVICE, ACCOUNT).is_err(),
            "corrupt credential must fail closed"
        );
    }
    for suffix in ["", "\n", "\r\n"] {
        fs::write(&path, format!("fixture-value{suffix}")).unwrap();
        assert_eq!(
            store.get_secret(SERVICE, ACCOUNT).unwrap().as_deref(),
            Some("fixture-value")
        );
    }
}

#[test]
fn failed_secret_rename_cleans_only_its_private_temp() {
    let home = Fixture::new();
    fs::create_dir_all(home.secrets().join("openrouter.secret")).unwrap();
    assert!(
        FileSecretStore::for_home(&home.0)
            .set_secret(SERVICE, ACCOUNT, "fixture-value")
            .is_err()
    );
    let names: Vec<_> = fs::read_dir(home.secrets())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names, ["openrouter.secret"]);
}

#[test]
fn symlinked_helpers_directory_never_changes_outside_files_or_permissions() {
    let home = Fixture::new();
    let outside = home.0.join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::set_permissions(&outside, fs::Permissions::from_mode(0o755)).unwrap();
    let original = outside.join("openrouter.sh");
    fs::write(&original, "original outside file").unwrap();
    let helpers = default_helpers_dir(&home.0);
    fs::create_dir_all(helpers.parent().unwrap()).unwrap();
    symlink(&outside, &helpers).unwrap();
    assert!(install_helper_script(&helpers, "openrouter", "fixture script").is_err());
    assert!(uninstall_helper_script(&helpers, "openrouter").is_err());
    assert_eq!(
        fs::read_to_string(original).unwrap(),
        "original outside file"
    );
    assert_eq!(mode(&outside), 0o755);
}

#[test]
fn symlinked_owned_root_is_rejected_before_chmod_or_install() {
    let home = Fixture::new();
    let outside = home.0.join("outside");
    fs::create_dir_all(outside.join("helpers")).unwrap();
    fs::set_permissions(&outside, fs::Permissions::from_mode(0o755)).unwrap();
    symlink(&outside, home.0.join(".claude-config-manager")).unwrap();
    assert!(
        install_helper_script(
            &default_helpers_dir(&home.0),
            "openrouter",
            "fixture script"
        )
        .is_err()
    );
    assert_eq!(mode(&outside), 0o755);
    assert!(!outside.join("helpers/openrouter.sh").exists());
}

#[test]
fn helper_install_is_private_and_failed_rename_leaves_no_owned_temp() {
    let home = Fixture::new();
    let helpers = default_helpers_dir(&home.0);
    let script = generate_helper_script("openrouter").unwrap();
    let path = install_helper_script(&helpers, "openrouter", &script).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), script);
    assert_eq!(mode(&path), 0o700);
    assert_eq!(mode(&helpers), 0o700);
    assert_eq!(mode(helpers.parent().unwrap()), 0o700);
    fs::create_dir(helpers.join("deepseek.sh")).unwrap();
    assert!(install_helper_script(&helpers, "deepseek", &script).is_err());
    assert!(
        !fs::read_dir(&helpers).unwrap().any(|e| e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".tmp-"))
    );
}

#[test]
fn legitimate_symlinked_home_ancestor_remains_supported() {
    let home = Fixture::new();
    let real_home = home.0.join("real-home");
    fs::create_dir_all(&real_home).unwrap();
    let alias = home.0.join("home-alias");
    symlink(&real_home, &alias).unwrap();
    let store = FileSecretStore::for_home(&alias);
    store.set_secret(SERVICE, ACCOUNT, "fixture-value").unwrap();
    assert_eq!(
        store.get_secret(SERVICE, ACCOUNT).unwrap().as_deref(),
        Some("fixture-value")
    );
    let path = install_helper_script(&default_helpers_dir(&alias), "openrouter", "fixture script")
        .unwrap();
    assert_eq!(fs::read_to_string(path).unwrap(), "fixture script");
}
