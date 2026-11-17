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

/// Production `CLAUDE_CONFIG_DIR` override, read once per call site.
///
/// Claude Code itself honors this documented variable (T02 config matrix), so
/// the app must read and write the same settings file Claude Code uses —
/// editing `~/.claude/settings.json` while the CLI runs from
/// `$CLAUDE_CONFIG_DIR/settings.json` would report success against the wrong
/// file. Empty/whitespace values mean unset, matching the CLI's behavior.
pub fn production_config_dir() -> Option<PathBuf> {
    std::env::var("CLAUDE_CONFIG_DIR")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

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

    /// Check every ancestor of `path` up to (but excluding) `root` for
    /// symlinks. A symlinked ancestor (e.g. `~/.claude` itself) would
    /// redirect reads AND writes outside the expected location (threat T5,
    /// T31 hardening) — returns the first offending directory.
    ///
    /// `root` must be an ancestor of `path` (typically the home dir).
    pub fn first_symlinked_ancestor(
        &self,
        path: &Path,
        root: &Path,
    ) -> Result<Option<PathBuf>, std::io::Error> {
        let mut cur = path.parent();
        while let Some(dir) = cur {
            if dir == root || dir.parent().is_none() {
                break;
            }
            let meta = std::fs::symlink_metadata(dir)?;
            if meta.file_type().is_symlink() {
                return Ok(Some(dir.to_path_buf()));
            }
            cur = dir.parent();
        }
        Ok(None)
    }

    /// Refuse symlinks in every user-controlled ancestor. HOME is a trusted
    /// boundary (home mounts themselves can legitimately be aliases). For
    /// external fixture/override paths we walk to root and only exempt Apple's
    /// root-owned /tmp, /var and /etc aliases to their exact /private targets.
    /// Missing first-run ancestors are allowed, but existing outer ancestors
    /// are still inspected before any directory creation.
    pub fn config_dir_is_symlink(&self, settings_path: &Path) -> Result<bool, std::io::Error> {
        let absolute = if settings_path.is_absolute() {
            settings_path.to_path_buf()
        } else {
            std::env::current_dir()?.join(settings_path)
        };
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute());
        let trusted_home = home.as_deref().filter(|home| absolute.starts_with(home));
        untrusted_symlinked_ancestor(&absolute, trusted_home)
    }
}

fn untrusted_symlinked_ancestor(
    path: &Path,
    trusted_home: Option<&Path>,
) -> Result<bool, std::io::Error> {
    let mut current = path.parent();
    while let Some(dir) = current {
        if Some(dir) == trusted_home || dir.parent().is_none() {
            break;
        }
        match std::fs::symlink_metadata(dir) {
            Ok(meta) if meta.file_type().is_symlink() => {
                if !is_system_alias(dir, &meta) {
                    return Ok(true);
                }
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        current = dir.parent();
    }
    Ok(false)
}

#[cfg(target_os = "macos")]
fn is_system_alias(path: &Path, meta: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    if meta.uid() != 0 {
        return false;
    }
    let target = match path.to_str() {
        Some("/tmp") => Path::new("/private/tmp"),
        Some("/var") => Path::new("/private/var"),
        Some("/etc") => Path::new("/private/etc"),
        _ => return false,
    };
    std::fs::read_link(path).ok().is_some_and(|actual| {
        actual == target
            || path
                .parent()
                .is_some_and(|parent| parent.join(actual) == target)
    })
}

#[cfg(not(target_os = "macos"))]
fn is_system_alias(_path: &Path, _meta: &std::fs::Metadata) -> bool {
    false
}

/// Open only a regular final file and cap the actual read, not just its initial
/// metadata. O_NONBLOCK prevents a raced replacement with a FIFO from hanging
/// before metadata can be checked; it has no effect on normal regular files.
pub(crate) fn read_regular_file_capped(path: &Path, limit: u64) -> std::io::Result<Vec<u8>> {
    use std::io::{Error, ErrorKind, Read};
    if ConfigLocator::new().config_dir_is_symlink(path)? {
        return Err(Error::new(
            ErrorKind::PermissionDenied,
            "source has a symlinked ancestor",
        ));
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        use std::os::unix::fs::OpenOptionsExt;
        #[cfg(target_os = "macos")]
        const SAFETY_FLAGS: i32 = 0x100 | 0x4; // O_NOFOLLOW | O_NONBLOCK
        #[cfg(target_os = "linux")]
        const SAFETY_FLAGS: i32 = 0x20000 | 0x800;
        options.custom_flags(SAFETY_FLAGS);
    }
    let file = options.open(path)?;
    let meta = file.metadata()?;
    if !meta.is_file() {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "source is not a regular file",
        ));
    }
    if meta.len() > limit {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "source exceeds the settings size limit",
        ));
    }
    let mut bytes = Vec::new();
    file.take(limit.saturating_add(1)).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "source exceeds the settings size limit",
        ));
    }
    Ok(bytes)
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
    fn trusted_home_alias_is_allowed_but_nested_alias_is_refused() {
        let root = temp_home("trusted-home");
        let real_home = root.join("real-home");
        let alias = root.join("home-alias");
        fs::create_dir_all(real_home.join("nested")).unwrap();
        symlink(&real_home, &alias).unwrap();
        assert!(
            !untrusted_symlinked_ancestor(&alias.join("nested/settings.json"), Some(&alias))
                .unwrap()
        );
        symlink(&root, real_home.join("nested/link")).unwrap();
        assert!(
            untrusted_symlinked_ancestor(&alias.join("nested/link/settings.json"), Some(&alias))
                .unwrap()
        );
        fs::remove_dir_all(root).unwrap();
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
