use claude_config_manager_lib::switch_history::{Outcome, append, read};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
};
#[test]
fn history_is_bounded_private_durable_and_does_not_overwrite_corruption() {
    let home = std::env::temp_dir().join(format!("ccm-history-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    for _ in 0..105 {
        append(&home, "nvidia-nim", Outcome::Auth).unwrap();
    }
    let events = read(&home).unwrap();
    assert_eq!(events.len(), 100);
    assert!(
        events
            .iter()
            .all(|e| e.provider_id == "nvidia-nim" && e.outcome == Outcome::Auth)
    );
    let file = home.join(".claude-config-manager/activity/history.json");
    assert_eq!(
        fs::metadata(&file).unwrap().permissions().mode() & 0o777,
        0o600
    );
    fs::write(&file, b"corrupt").unwrap();
    assert!(append(&home, "nvidia-nim", Outcome::Selected).is_err());
    assert_eq!(fs::read(&file).unwrap(), b"corrupt");
    fs::remove_file(&file).unwrap();
    let outside = home.join("outside");
    fs::write(&outside, b"preserve").unwrap();
    symlink(&outside, &file).unwrap();
    assert!(append(&home, "nvidia-nim", Outcome::Auth).is_err());
    assert_eq!(fs::read(&outside).unwrap(), b"preserve");
    fs::remove_dir_all(home).unwrap();
}
