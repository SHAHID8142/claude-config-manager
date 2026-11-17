//! Private before-image journal for managed-file compensation after an
//! interrupted operation. Recovery is explicit: it never silently overwrites
//! files edited after a crash. This is not a multi-file filesystem transaction.
use crate::apply_lock::ApplyLock;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const VERSION: u32 = 1;
const RAW_LIMIT: usize = 32 * 1024 * 1024;
const JOURNAL_LIMIT: u64 = 48 * 1024 * 1024;
const FILE_LIMIT: u64 = 10 * 1024 * 1024;
const GROUP_LIMIT: usize = 300;
const MANAGED_METADATA: [&str; 3] = ["credentials.json", "routing.json", "profiles.json"];

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Image {
    path: PathBuf,
    /// None means the file did not exist before the operation.
    bytes_base64: Option<String>,
    mode: u32,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Journal {
    version: u32,
    id: String,
    created_epoch: u64,
    settings_path: Option<PathBuf>,
    files: Vec<Image>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingSummary {
    pub created_epoch: u64,
    pub file_count: usize,
    pub settings_path: Option<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryOutcome {
    pub restored_previous: bool,
    pub file_count: usize,
    pub current_snapshot_file: Option<String>,
}

pub struct ManagedTransaction {
    home: PathBuf,
    journal: Journal,
    lock: ApplyLock,
}
fn root(home: &Path) -> PathBuf {
    home.join(".claude-config-manager")
}
pub fn journal_path(home: &Path) -> PathBuf {
    root(home).join("apply-journal.json")
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn nonce() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}
fn lock_for(home: &Path) -> Result<ApplyLock, String> {
    let backups = crate::config_backup::default_backup_dir(home);
    let directory = backups
        .parent()
        .ok_or("Recovery lock directory is invalid")?;
    guard_directories(home, directory)?;
    ApplyLock::acquire(directory).map_err(|e| e.to_string())
}
/// Only owned ancestors are checked for in-home paths. Legitimate symlinked
/// /tmp, /var or home ancestors on macOS do not redirect an app-owned child.
fn guard_directories(home: &Path, directory: &Path) -> Result<(), String> {
    let inside = directory.starts_with(home);
    let mut current = Some(directory);
    while let Some(path) = current {
        if inside && path == home {
            break;
        }
        if path == Path::new("/") {
            break;
        }
        let system_alias = !inside && [Path::new("/tmp"), Path::new("/var")].contains(&path);
        match fs::symlink_metadata(path) {
            Ok(m) if !system_alias && (m.file_type().is_symlink() || !m.is_dir()) => {
                return Err(
                    "Managed directory is not a regular directory; symlinks are refused".into(),
                );
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("Cannot inspect managed directory ({e})")),
        }
        current = path.parent();
    }
    Ok(())
}
fn safe_component(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && !value.starts_with('-')
        && value
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}
fn clean_absolute(path: &Path) -> bool {
    path.is_absolute()
        && !path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
}
fn expected_mode(home: &Path, settings: Option<&Path>, path: &Path) -> Result<u32, String> {
    if !clean_absolute(path) {
        return Err("Recovery file path must be an absolute normalized path".into());
    }
    if settings == Some(path) {
        return Ok(0o600);
    }
    let relative = path
        .strip_prefix(root(home))
        .map_err(|_| "Recovery file is outside the managed scope")?;
    let components: Vec<_> = relative.components().collect();
    if components.len() == 1
        && MANAGED_METADATA
            .iter()
            .any(|name| relative == Path::new(name))
    {
        return Ok(0o600);
    }
    if components.len() == 2 {
        let directory = components[0].as_os_str();
        let name = components[1]
            .as_os_str()
            .to_str()
            .ok_or("Invalid managed filename")?;
        let suffix = if directory == "secrets" {
            ".secret"
        } else if directory == "helpers" {
            ".sh"
        } else {
            return Err("Unknown managed recovery directory".into());
        };
        if name.strip_suffix(suffix).is_some_and(safe_component) {
            return Ok(if suffix == ".sh" { 0o700 } else { 0o600 });
        }
    }
    Err("Recovery file is outside the allowed managed-file set".into())
}
fn read_file(
    home: &Path,
    path: &Path,
    limit: u64,
    private: bool,
) -> Result<Option<Vec<u8>>, String> {
    let directory = path.parent().ok_or("Managed file has no parent")?;
    guard_directories(home, directory)?;
    let metadata = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("Cannot inspect managed file ({e})")),
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("Managed file is not a regular file; symlinks are refused".into());
    }
    if metadata.len() > limit {
        return Err("Managed file exceeds the recovery size limit".into());
    }
    if private && metadata.permissions().mode() & 0o777 != 0o600 {
        return Err("Recovery journal must have owner-only 0600 permissions".into());
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(target_os = "macos")]
    options.custom_flags(0x100);
    #[cfg(target_os = "linux")]
    options.custom_flags(0x20000);
    let file = options
        .open(path)
        .map_err(|e| format!("Cannot open managed file ({e})"))?;
    let metadata = file
        .metadata()
        .map_err(|e| format!("Cannot inspect opened managed file ({e})"))?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err("Opened managed file is invalid or too large".into());
    }
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Cannot read managed file ({e})"))?;
    if bytes.len() as u64 > limit {
        return Err("Managed file exceeds the recovery size limit".into());
    }
    Ok(Some(bytes))
}
fn image(home: &Path, settings: Option<&Path>, path: &Path) -> Result<Image, String> {
    let mode = expected_mode(home, settings, path)?;
    let bytes = read_file(home, path, FILE_LIMIT, false)?;
    Ok(Image {
        path: path.to_path_buf(),
        bytes_base64: bytes.map(|b| STANDARD.encode(b)),
        mode,
    })
}
fn snapshot(home: &Path, settings: Option<&Path>) -> Result<Journal, String> {
    if settings.is_some_and(|p| !clean_absolute(p)) {
        return Err("Configured settings path must be absolute and normalized".into());
    }
    let mut files = Vec::new();
    if let Some(path) = settings {
        files.push(image(home, settings, path)?);
    }
    for name in MANAGED_METADATA {
        files.push(image(home, settings, &root(home).join(name))?);
    }
    for (directory, suffix) in [("helpers", ".sh"), ("secrets", ".secret")] {
        let dir = root(home).join(directory);
        guard_directories(home, &dir)?;
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(format!("Cannot list managed directory ({e})")),
        };
        let mut count = 0;
        for entry in entries {
            let entry = entry.map_err(|e| format!("Cannot inspect managed entry ({e})"))?;
            if !entry.file_name().to_string_lossy().ends_with(suffix) {
                continue;
            }
            count += 1;
            if count > GROUP_LIMIT {
                return Err("Too many managed files for recovery".into());
            }
            files.push(image(home, settings, &entry.path())?);
        }
    }
    let journal = Journal {
        version: VERSION,
        id: crate::proxy::random_token(),
        created_epoch: now(),
        settings_path: settings.map(Path::to_path_buf),
        files,
    };
    validate(home, settings, &journal)?;
    Ok(journal)
}
fn validate(home: &Path, settings: Option<&Path>, journal: &Journal) -> Result<(), String> {
    if journal.version != VERSION
        || journal.id.len() != 64
        || !journal.id.chars().all(|c| c.is_ascii_hexdigit())
    {
        return Err("Recovery journal version or identifier is invalid".into());
    }
    if journal
        .settings_path
        .as_deref()
        .is_some_and(|p| Some(p) != settings)
    {
        return Err("Recovery journal belongs to a different configured settings path".into());
    }
    if journal.files.len() > GROUP_LIMIT * 2 + MANAGED_METADATA.len() + 1 {
        return Err("Too many recovery entries".into());
    }
    let mut paths = HashSet::new();
    let mut total = 0usize;
    let mut secrets = 0;
    let mut helpers = 0;
    for entry in &journal.files {
        if entry.mode != expected_mode(home, journal.settings_path.as_deref(), &entry.path)?
            || !paths.insert(entry.path.clone())
        {
            return Err("Recovery journal contains invalid modes or duplicate paths".into());
        }
        if entry.path.parent() == Some(root(home).join("secrets").as_path()) {
            secrets += 1;
        }
        if entry.path.parent() == Some(root(home).join("helpers").as_path()) {
            helpers += 1;
        }
        if let Some(encoded) = &entry.bytes_base64 {
            if encoded.len() > (FILE_LIMIT as usize).div_ceil(3) * 4 {
                return Err("Recovery image exceeds file size limit".into());
            }
            let bytes = STANDARD
                .decode(encoded)
                .map_err(|_| "Recovery journal has an invalid byte image")?;
            if bytes.len() as u64 > FILE_LIMIT {
                return Err("Recovery image exceeds file size limit".into());
            }
            total += bytes.len();
            if total > RAW_LIMIT {
                return Err("Managed recovery state exceeds its size limit".into());
            }
        }
    }
    if secrets > GROUP_LIMIT || helpers > GROUP_LIMIT {
        return Err("Too many managed recovery files".into());
    }
    Ok(())
}
fn sync_directory(path: &Path) -> Result<(), String> {
    File::open(path)
        .and_then(|f| f.sync_all())
        .map_err(|e| format!("Cannot sync managed directory ({e})"))
}
fn write_private(
    home: &Path,
    path: &Path,
    bytes: &[u8],
    mode: u32,
    replace: bool,
) -> Result<(), String> {
    let parent = path.parent().ok_or("Managed file has no parent")?;
    guard_directories(home, parent)?;
    match fs::symlink_metadata(path) {
        Ok(m) if !m.is_file() || m.file_type().is_symlink() => {
            return Err("Managed destination is not a regular file".into());
        }
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("Cannot inspect managed destination ({e})")),
    }
    fs::create_dir_all(parent).map_err(|e| format!("Cannot create managed directory ({e})"))?;
    if parent.starts_with(root(home)) {
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
            .map_err(|e| format!("Cannot protect managed directory ({e})"))?;
    }
    let tmp = parent.join(format!(".ccm-recovery-{}-{}", std::process::id(), nonce()));
    let result = (|| {
        let mut f = OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(mode)
            .open(&tmp)
            .map_err(|e| format!("Cannot create private recovery temporary file ({e})"))?;
        f.write_all(bytes)
            .and_then(|_| f.sync_all())
            .map_err(|e| format!("Cannot persist private recovery file ({e})"))?;
        if replace {
            fs::rename(&tmp, path)
        } else {
            fs::hard_link(&tmp, path)
        }
        .map_err(|e| format!("Cannot install recovery file ({e})"))?;
        sync_directory(parent)
    })();
    let _ = fs::remove_file(&tmp);
    result
}
fn document(home: &Path, journal: &Journal, path: &Path, replace: bool) -> Result<(), String> {
    validate(home, journal.settings_path.as_deref(), journal)?;
    let raw = serde_json::to_vec(journal).map_err(|_| "Cannot serialize recovery journal")?;
    if raw.len() as u64 > JOURNAL_LIMIT {
        return Err("Recovery journal exceeds its size limit".into());
    }
    write_private(home, path, &raw, 0o600, replace)
}
fn read_journal(home: &Path) -> Result<Option<Journal>, String> {
    let Some(raw) = read_file(home, &journal_path(home), JOURNAL_LIMIT, true)? else {
        return Ok(None);
    };
    serde_json::from_slice(&raw)
        .map(Some)
        .map_err(|_| "Recovery journal is invalid JSON or schema".into())
}
fn remove_marker(home: &Path, id: &str) -> Result<(), String> {
    let current = read_journal(home)?.ok_or("Recovery journal disappeared before completion")?;
    if current.id != id {
        return Err("Recovery journal changed ownership; it was retained".into());
    }
    fs::remove_file(journal_path(home))
        .map_err(|e| format!("Cannot clear recovery journal ({e})"))?;
    sync_directory(&root(home))
}
fn restore_images(home: &Path, journal: &Journal) -> Result<(), String> {
    let mut failures = Vec::new();
    for (index, entry) in journal.files.iter().enumerate() {
        let result = (|| {
            expected_mode(home, journal.settings_path.as_deref(), &entry.path)?;
            let parent = entry.path.parent().ok_or("Managed path has no parent")?;
            guard_directories(home, parent)?;
            if let Some(encoded) = &entry.bytes_base64 {
                let bytes = STANDARD
                    .decode(encoded)
                    .map_err(|_| "Recovery byte image is invalid")?;
                write_private(home, &entry.path, &bytes, entry.mode, true)?;
                let verify = read_file(home, &entry.path, FILE_LIMIT, false)?
                    .ok_or("Recovered managed file is missing")?;
                if verify != bytes {
                    return Err("Recovered managed file did not verify".into());
                }
            } else {
                match fs::symlink_metadata(&entry.path) {
                    Ok(m) if !m.is_file() || m.file_type().is_symlink() => {
                        return Err("New managed file became an unsafe destination".into());
                    }
                    Ok(_) => {
                        fs::remove_file(&entry.path)
                            .map_err(|e| format!("Cannot remove new managed file ({e})"))?;
                        sync_directory(parent)?;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => return Err(format!("Cannot inspect new managed file ({e})")),
                }
            }
            Ok(())
        })();
        if let Err(error) = result {
            failures.push(format!("entry {}: {error}", index + 1));
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Managed rollback was incomplete; recovery journal retained. {}",
            failures.join("; ")
        ))
    }
}
impl ManagedTransaction {
    pub fn begin(home: &Path, settings: Option<&Path>) -> Result<Self, String> {
        if !clean_absolute(home) {
            return Err("Managed home must be an absolute normalized path".into());
        }
        let lock = lock_for(home)?;
        guard_directories(home, &root(home))?;
        if read_journal(home)?.is_some() {
            return Err("An interrupted managed operation needs recovery before another change. Open Recovery & diagnostics.".into());
        }
        let journal = snapshot(home, settings)?;
        document(home, &journal, &journal_path(home), false)?;
        Ok(Self {
            home: home.to_path_buf(),
            journal,
            lock,
        })
    }
    pub fn lock(&self) -> &ApplyLock {
        &self.lock
    }
    pub fn track(&mut self, path: &Path) -> Result<(), String> {
        expected_mode(&self.home, self.journal.settings_path.as_deref(), path)?;
        if self.journal.files.iter().any(|image| image.path == path) {
            return Ok(());
        }
        let next = image(&self.home, self.journal.settings_path.as_deref(), path)?;
        self.journal.files.push(next);
        validate(
            &self.home,
            self.journal.settings_path.as_deref(),
            &self.journal,
        )?;
        let current = read_journal(&self.home)?
            .ok_or("Recovery journal disappeared before a managed write")?;
        if current.id != self.journal.id {
            return Err("Recovery journal changed ownership".into());
        }
        document(&self.home, &self.journal, &journal_path(&self.home), true)
    }
    pub fn commit(self) -> Result<(), String> {
        remove_marker(&self.home, &self.journal.id)
    }
    /// Settings writes already compensate themselves. Preserve an external
    /// editor's current settings when an Apply failed before/at its commit.
    pub fn rollback_preserving_settings(self, retain_marker: bool) -> Result<(), String> {
        let current =
            read_journal(&self.home)?.ok_or("Recovery journal disappeared before rollback")?;
        if current.id != self.journal.id {
            return Err("Recovery journal changed ownership".into());
        }
        let mut journal = self.journal;
        if let Some(settings) = journal.settings_path.as_ref() {
            journal.files.retain(|file| &file.path != settings);
        }
        restore_images(&self.home, &journal)?;
        if retain_marker {
            return Err("Managed rollback needs explicit recovery; journal retained".into());
        }
        remove_marker(&self.home, &journal.id)
    }
    pub fn rollback(self) -> Result<(), String> {
        let current =
            read_journal(&self.home)?.ok_or("Recovery journal disappeared before rollback")?;
        if current.id != self.journal.id {
            return Err("Recovery journal changed ownership; rollback was not attempted".into());
        }
        restore_images(&self.home, &self.journal)?;
        remove_marker(&self.home, &self.journal.id)
    }
}
/// Dropping the object releases only its lock. The marker is deliberately left
/// on disk, including during panic unwinding. No fallible recovery is hidden in
/// Drop and a crash cannot turn an incomplete operation into a success signal.
pub fn pending(home: &Path) -> Result<Option<PendingSummary>, String> {
    let Some(journal) = read_journal(home)? else {
        return Ok(None);
    };
    validate(home, journal.settings_path.as_deref(), &journal)?;
    Ok(Some(PendingSummary {
        created_epoch: journal.created_epoch,
        file_count: journal.files.len(),
        settings_path: journal
            .settings_path
            .map(|p| p.to_string_lossy().to_string()),
    }))
}
pub fn recover(
    home: &Path,
    settings: Option<&Path>,
    restore_previous: bool,
) -> Result<RecoveryOutcome, String> {
    let _lock = lock_for(home)?;
    let journal =
        read_journal(home)?.ok_or("There is no interrupted managed operation to recover")?;
    validate(home, settings, &journal)?;
    // Before either explicit decision, validate live destinations without
    // following symlinks. Preserve current bytes before any restoration.
    let mut current = Journal {
        version: VERSION,
        id: crate::proxy::random_token(),
        created_epoch: now(),
        settings_path: journal.settings_path.clone(),
        files: Vec::new(),
    };
    for entry in &journal.files {
        current
            .files
            .push(image(home, journal.settings_path.as_deref(), &entry.path)?);
    }
    let snapshot_name =
        if restore_previous {
            let name = format!("recovery-before-{}-{}.json", std::process::id(), nonce());
            document(home, &current, &root(home).join(&name), false)?;
            restore_images(home, &journal)?;
            Some(name)
        } else {
            if let Some(path) = journal.settings_path.as_deref() {
                match crate::config_reader::read_settings(path) {
                    Ok(_) | Err(crate::config_reader::ConfigError::Missing) => {}
                    Err(_) => return Err(
                        "Current settings are invalid; restore the previous managed state instead"
                            .into(),
                    ),
                }
            }
            for name in MANAGED_METADATA {
                let path = root(home).join(name);
                if path.exists() && crate::config_reader::read_settings(&path).is_err() {
                    return Err(
                        "Current managed metadata is invalid; restore the previous state instead"
                            .into(),
                    );
                }
            }
            None
        };
    remove_marker(home, &journal.id)?;
    Ok(RecoveryOutcome {
        restored_previous: restore_previous,
        file_count: journal.files.len(),
        current_snapshot_file: snapshot_name,
    })
}
