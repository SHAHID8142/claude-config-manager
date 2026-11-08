//! Read-only global Claude Code configuration locator (T26).
//!
//! Facts (docs/research/claude-code-config-matrix.md, T02):
//! - User-scope settings live at `~/.claude/settings.json` (write target of
//!   this app; never `~/.claude.json`).
//! - `CLAUDE_CONFIG_DIR` relocates the whole Claude directory — the locator
//!   honors it via an explicit override parameter so the production caller
//!   reads the env once and tests stay race-free.
//!
//! Safety: symlinks on the settings path are REFUSED (threat T5) — the
//! locator reports `Symlink` and downstream readers/writers treat it as an
//! error, never a write target.

use std::path::{Path, PathBuf};

/// What exists at the located settings path (lstat-based; no content read).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocatedStatus {
    /// Directory or file does not exist — first-run state.
    Missing,
    /// A regular file (readable content handled by the reader task, T27).
    RegularFile,
    /// The path is a symlink — refused for reads/writes (threat T5).
    Symlink,
    /// Exists but is neither regular file nor symlink (directory, fifo, …).
    OtherFile,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConfigLocator {
    settings_file_name: &'static str,
}

impl Default for ConfigLocator {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigLocator {
    pub fn new() -> Self {
        Self {
            settings_file_name: "settings.json",
        }
    }

    /// Compute the user-scope settings path.
    ///
    /// `config_dir_override` stands in for `CLAUDE_CONFIG_DIR` (the
    /// production caller passes `std::env::var("CLAUDE_CONFIG_DIR")` result;
    /// tests pass fixtures). Relative override values are resolved against
    /// `home`, mirroring how users set the variable.
    pub fn locate(&self, home: &Path, config_dir_override: Option<&Path>) -> PathBuf {
        let dir = match config_dir_override {
            Some(dir) if dir.is_absolute() => dir.to_path_buf(),
            Some(dir) => home.join(dir),
            None => home.join(".claude"),
        };
        dir.join(self.settings_file_name)
    }

    /// Inspect the located path without reading contents. Returns the
    /// symlink case distinctly so callers can refuse it.
    pub fn inspect(&self, path: &Path) -> Result<LocatedStatus, std::io::Error> {
        use std::os::unix::fs::FileTypeExt;
        let meta = match std::fs::symlink_metadata(path) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(LocatedStatus::Missing);
            }
            Err(e) => return Err(e),
        };
        let ft = meta.file_type();
        if ft.is_symlink() {
            Ok(LocatedStatus::Symlink)
        } else if ft.is_file() {
            Ok(LocatedStatus::RegularFile)
        } else if ft.is_dir() {
            // A directory at the settings path is unusable — surface it.
            Ok(LocatedStatus::OtherFile)
        } else {
            let _ = FileTypeExt::is_fifo(&ft); // keep unix import meaningful
            Ok(LocatedStatus::OtherFile)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::symlink;

    fn temp_home(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ccm-t26-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("create temp home");
        dir
    }

    #[test]
    fn locates_default_user_settings_path() {
        let home = temp_home("default");
        let locator = ConfigLocator::new();
        let path = locator.locate(&home, None);
        assert_eq!(path, home.join(".claude").join("settings.json"));
        assert_eq!(
            locator.inspect(&path).expect("inspect"),
            LocatedStatus::Missing
        );
        fs::remove_dir_all(&home).expect("cleanup");
    }

    #[test]
    fn honors_absolute_config_dir_override() {
        let home = temp_home("abs");
        let custom = temp_home("abs-dir");
        let locator = ConfigLocator::new();
        let path = locator.locate(&home, Some(&custom));
        assert_eq!(path, custom.join("settings.json"));
        assert_ne!(path, home.join(".claude").join("settings.json"));
        fs::remove_dir_all(&home).expect("cleanup");
        fs::remove_dir_all(&custom).expect("cleanup");
    }

    #[test]
    fn relative_override_resolves_against_home() {
        let home = temp_home("rel");
        let locator = ConfigLocator::new();
        let path = locator.locate(&home, Some(Path::new("custom-claude")));
        assert_eq!(path, home.join("custom-claude").join("settings.json"));
        fs::remove_dir_all(&home).expect("cleanup");
    }

    #[test]
    fn inspect_reports_regular_file() {
        let home = temp_home("regular");
        let locator = ConfigLocator::new();
        let path = locator.locate(&home, None);
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(&path, "{}").expect("write fixture");
        assert_eq!(
            locator.inspect(&path).expect("inspect"),
            LocatedStatus::RegularFile
        );
        fs::remove_dir_all(&home).expect("cleanup");
    }

    #[test]
    fn inspect_refuses_symlink_target() {
        let home = temp_home("symlink");
        let locator = ConfigLocator::new();
        let path = locator.locate(&home, None);
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        let sensitive = home.join("innocent-victim.txt");
        fs::write(&sensitive, "secret").expect("victim file");
        symlink(&sensitive, &path).expect("create symlink");
        assert_eq!(
            locator.inspect(&path).expect("inspect"),
            LocatedStatus::Symlink
        );
        fs::remove_dir_all(&home).expect("cleanup");
    }

    #[test]
    fn inspect_reports_directory_at_settings_path() {
        let home = temp_home("dir");
        let locator = ConfigLocator::new();
        let path = locator.locate(&home, None);
        fs::create_dir_all(&path).expect("create dir at settings path");
        assert_eq!(
            locator.inspect(&path).expect("inspect"),
            LocatedStatus::OtherFile
        );
        fs::remove_dir_all(&home).expect("cleanup");
    }

    #[test]
    fn inspect_on_missing_parent_is_missing_not_error() {
        let home = temp_home("noparent");
        let locator = ConfigLocator::new();
        let path = locator.locate(&home, None);
        // .claude directory deliberately not created.
        assert_eq!(
            locator.inspect(&path).expect("inspect"),
            LocatedStatus::Missing
        );
        fs::remove_dir_all(&home).expect("cleanup");
    }
}
