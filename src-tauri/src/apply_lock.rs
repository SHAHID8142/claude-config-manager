//! G4 — single-instance apply lock.
//!
//! Two app windows, or a double-click on "Save & Apply", previously produced two
//! independent read‑merge‑write sequences over the same file. The mtime check
//! catches *some* of that, but only after both have read: two writers can pass
//! the check within the same second and the later one wins silently.
//!
//! This lock makes the whole pipeline mutually exclusive across processes:
//! - acquired with `create_new` (O_EXCL) — atomic on APFS and NFS-safe enough
//!   for a single-user desktop app;
//! - the file records the owning pid and a timestamp purely for diagnostics and
//!   stale detection;
//! - a lock whose owner is gone (or that is absurdly old) is reclaimed, so a
//!   crash cannot brick the app permanently;
//! - released by dropping the guard, and the file is owner-only (`0600`).

use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// A lock older than this is assumed orphaned even if the pid check is
/// inconclusive. An apply takes milliseconds; two minutes is generous.
const STALE_AFTER_NANOS: u128 = 120 * 1_000_000_000;

/// A very young lock is *always* treated as held, even if its contents cannot be
/// read yet.
///
/// `create_new` makes the file appear before we can write the pid into it, so for
/// a few microseconds a second process sees an empty file with no owner. Judging
/// that "owner is dead" would let two applies run at once — a real race that the
/// concurrency test caught. Failing closed during this grace window is correct:
/// no legitimate apply can be older than it and unreadable.
const RECENT_GRACE_NANOS: u128 = 5 * 1_000_000_000;

/// Held for the duration of an apply; removing it on drop releases the lock.
#[derive(Debug)]
pub struct ApplyLock {
    path: PathBuf,
    /// Unique per acquisition. Ownership is decided by this token, not by pid:
    /// two guards can exist inside one process (and a pid can be recycled).
    nonce: String,
}

impl ApplyLock {
    /// Take the lock at `<dir>/apply.lock`.
    pub fn acquire(dir: &Path) -> Result<Self, LockError> {
        fs::create_dir_all(dir).map_err(|e| LockError::Io(e.to_string()))?;
        let path = dir.join("apply.lock");

        match Self::try_create(&path) {
            Ok(nonce) => Ok(Self { path, nonce }),
            Err(LockError::Held { pid, age_nanos }) => {
                // A lock created moments ago is held, full stop — its owner may
                // simply not have written its pid yet.
                if age_nanos < RECENT_GRACE_NANOS {
                    return Err(LockError::Held { pid, age_nanos });
                }
                if !owner_is_alive(pid) || age_nanos > STALE_AFTER_NANOS {
                    // Orphaned by a crash/kill: reclaim once, then report failure
                    // normally if someone else got in first.
                    let _ = fs::remove_file(&path);
                    Self::try_create(&path).map(|nonce| Self { path, nonce })
                } else {
                    Err(LockError::Held { pid, age_nanos })
                }
            }
            Err(e) => Err(e),
        }
    }

    fn try_create(path: &Path) -> Result<String, LockError> {
        use std::io::ErrorKind;
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
        {
            Ok(mut file) => {
                let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
                let nonce = next_nonce();
                // Line 1 = pid (diagnostics/staleness), line 2 = ownership token.
                let _ = writeln!(file, "{}\n{}", std::process::id(), nonce);
                Ok(nonce)
            }
            Err(e) if e.kind() == ErrorKind::AlreadyExists => Err(read_holder(path)),
            Err(e) => Err(LockError::Io(e.to_string())),
        }
    }

    /// Path of the lock file (diagnostics/tests).
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ApplyLock {
    fn drop(&mut self) {
        // Only remove a lock we still own: if a stale-reclaim replaced it, the
        // token differs and we must not delete the new owner's file.
        if owner_token(&self.path).as_deref() == Some(self.nonce.as_str()) {
            let _ = fs::remove_file(&self.path);
        }
    }
}

/// Ownership token stored on line 2 of the lock file.
fn owner_token(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    text.lines().nth(1).map(|s| s.trim().to_string())
}

/// Monotonic-ish unique token without a random-number dependency.
fn next_nonce() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:x}-{:x}", nanos, seq)
}

#[derive(Debug)]
pub enum LockError {
    /// Another live apply owns the lock.
    Held {
        pid: Option<u32>,
        age_nanos: u128,
    },
    Io(String),
}

impl std::fmt::Display for LockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LockError::Held { .. } => write!(
                f,
                "another Save & Apply is already running — wait for it to finish"
            ),
            LockError::Io(m) => write!(f, "could not acquire the apply lock ({m})"),
        }
    }
}

fn read_holder(path: &Path) -> LockError {
    // Line 1 is the pid; line 2 is the ownership token.
    let pid = fs::read_to_string(path)
        .ok()
        .and_then(|s| s.lines().next().and_then(|l| l.trim().parse::<u32>().ok()));
    let age_nanos = fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| std::time::SystemTime::now().duration_since(t).ok())
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    LockError::Held { pid, age_nanos }
}

/// Is the recorded pid still running? `kill -0` via the absolute binary so a
/// hostile `PATH` cannot fake the answer.
fn owner_is_alive(pid: Option<u32>) -> bool {
    let Some(pid) = pid else { return false };
    std::process::Command::new("/bin/kill")
        .arg("-0")
        .arg(pid.to_string())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    /// Backdate a file's mtime (stale-lock simulation).
    fn age_by(path: &Path, secs: u64) {
        let when = SystemTime::now() - std::time::Duration::from_secs(secs);
        fs::File::options()
            .write(true)
            .open(path)
            .expect("open")
            .set_times(fs::FileTimes::new().set_modified(when))
            .expect("set mtime");
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ccm-g4-{tag}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("dir");
        dir
    }

    #[test]
    fn a_second_acquire_is_refused_while_the_first_is_held() {
        let dir = temp_dir("held");
        let first = ApplyLock::acquire(&dir).expect("first lock");
        let err = ApplyLock::acquire(&dir).expect_err("second acquire must fail");
        assert!(matches!(err, LockError::Held { .. }));
        assert!(err.to_string().contains("already running"));
        drop(first);
        ApplyLock::acquire(&dir).expect("lock is free after release");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn dropping_the_guard_releases_the_lock_file() {
        let dir = temp_dir("release");
        let lock = ApplyLock::acquire(&dir).expect("lock");
        let path = lock.path().to_path_buf();
        assert!(path.exists());
        assert_eq!(
            fs::metadata(&path).expect("meta").permissions().mode() & 0o777,
            0o600,
            "lock file must be owner-only"
        );
        drop(lock);
        assert!(!path.exists(), "lock file must be gone after release");
        fs::remove_dir_all(&dir).ok();
    }

    /// The race that broke the concurrency test: a lock file that exists but has
    /// no readable pid yet must be treated as HELD, never as ownerless.
    #[test]
    fn an_empty_but_fresh_lock_is_held_not_stolen() {
        let dir = temp_dir("fresh-empty");
        fs::write(dir.join("apply.lock"), "").expect("seed empty lock");
        let err = ApplyLock::acquire(&dir).expect_err("must not steal a fresh lock");
        assert!(matches!(err, LockError::Held { .. }));
        assert!(
            dir.join("apply.lock").exists(),
            "the existing lock file must not be deleted"
        );
        fs::remove_dir_all(&dir).ok();
    }

    /// ...and an unreadable lock that is genuinely old is still reclaimable, so a
    /// crash cannot wedge the app forever.
    #[test]
    fn an_empty_and_old_lock_is_reclaimed() {
        let dir = temp_dir("old-empty");
        let path = dir.join("apply.lock");
        fs::write(&path, "").expect("seed");
        let old = SystemTime::now() - std::time::Duration::from_secs(3600);
        fs::File::options()
            .write(true)
            .open(&path)
            .expect("open")
            .set_times(fs::FileTimes::new().set_modified(old))
            .expect("mtime");
        ApplyLock::acquire(&dir).expect("an old empty lock must be reclaimed");
        fs::remove_dir_all(&dir).ok();
    }

    /// A lock from a dead process is reclaimed — after the grace window, which
    /// exists so a freshly created lock cannot be stolen mid-write.
    #[test]
    fn a_lock_from_a_dead_process_is_reclaimed() {
        let dir = temp_dir("stale-dead");
        let path = dir.join("apply.lock");
        // pid 999_999 is above macOS' default maxproc, so it cannot be alive.
        fs::write(&path, "999999\nnonce\n").expect("seed stale lock");
        age_by(&path, 10);
        ApplyLock::acquire(&dir).expect("stale lock must be reclaimed");
        fs::remove_dir_all(&dir).ok();
    }

    /// Within the grace window the same dead-pid lock is still respected: the
    /// worst case is a few seconds of "another apply is running", never two
    /// writers.
    #[test]
    fn a_dead_pid_lock_is_still_respected_inside_the_grace_window() {
        let dir = temp_dir("dead-fresh");
        fs::write(dir.join("apply.lock"), "999999\nnonce\n").expect("seed");
        let err = ApplyLock::acquire(&dir).expect_err("grace window must fail closed");
        assert!(matches!(err, LockError::Held { .. }));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn an_ancient_lock_is_reclaimed_even_if_the_pid_looks_alive() {
        let dir = temp_dir("stale-old");
        let path = dir.join("apply.lock");
        // Our own pid: alive, but the timestamp makes the lock orphaned.
        fs::write(&path, format!("{}\n", std::process::id())).expect("seed");
        let old = SystemTime::now() - std::time::Duration::from_secs(3600);
        let file = fs::File::options().write(true).open(&path).expect("open");
        file.set_times(fs::FileTimes::new().set_modified(old))
            .expect("set mtime");
        ApplyLock::acquire(&dir).expect("ancient lock must be reclaimed");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_holder_is_not_deleted_by_a_stale_reclaimer_that_lost() {
        // Guard against the classic bug: A holds, B reclaims A's lock, A drops and
        // deletes B's lock, leaving the file unlocked while B is still working.
        let dir = temp_dir("no-cross-delete");
        let a = ApplyLock::acquire(&dir).expect("a");
        fs::write(a.path(), "999999\nnonce-a\n").expect("simulate a's pid being dead");
        age_by(a.path(), 10);
        // Someone else takes over the (now stale) lock.
        let b = ApplyLock::acquire(&dir).expect("b reclaims");
        drop(a);
        assert!(
            b.path().exists(),
            "the losing holder must not delete the winner's lock"
        );
        drop(b);
        fs::remove_dir_all(&dir).ok();
    }
}
