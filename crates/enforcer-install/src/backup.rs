//! Pre-write backup-and-restore helpers so a failed/aborted install never
//! leaves a harness config corrupted. Every adapter calls
//! [`backup_before_write`] immediately before mutating an existing file
//! and, on any subsequent failure, [`restore`] to put the original bytes
//! back.
//!
//! The backup path convention is `{original}.enforcer-bak` (single fixed
//! suffix, never a timestamp-suffixed pile — an adapter that runs twice in
//! a row overwrites its own prior backup, which is the correct behavior
//! for "the last known-good state before the most recent write attempt").

use std::path::{Path, PathBuf};

use crate::error::{InstallError, InstallResult};

/// Suffix appended to a config path to form its backup path.
pub const BACKUP_SUFFIX: &str = ".enforcer-bak";

/// The backup path for a given original config path.
#[must_use]
pub fn backup_path_for(original: &Path) -> PathBuf {
    let mut backup = original.as_os_str().to_owned();
    backup.push(BACKUP_SUFFIX);
    PathBuf::from(backup)
}

/// Copy `original` to its backup path before an adapter overwrites it.
/// A no-op (returns `Ok(None)`) when `original` does not exist yet — a
/// fresh install has nothing to back up.
///
/// # Errors
/// Returns [`InstallError::BackupFailed`] if `original` exists but the
/// copy fails (permissions, disk full, etc).
pub fn backup_before_write(original: &Path) -> InstallResult<Option<PathBuf>> {
    if !original.exists() {
        return Ok(None);
    }
    let backup = backup_path_for(original);
    std::fs::copy(original, &backup).map_err(|e| InstallError::BackupFailed {
        path: original.display().to_string(),
        reason: e.to_string(),
    })?;
    Ok(Some(backup))
}

/// Restore `original` from its backup path, e.g. after a failed
/// multi-step apply. Leaves the backup file in place (idempotent restore:
/// calling this twice in a row is safe).
///
/// # Errors
/// Returns [`InstallError::BackupFailed`] if no backup exists for
/// `original`, or the restore copy fails.
pub fn restore(original: &Path) -> InstallResult<()> {
    let backup = backup_path_for(original);
    if !backup.exists() {
        return Err(InstallError::BackupFailed {
            path: original.display().to_string(),
            reason: format!("no backup found at `{}`", backup.display()),
        });
    }
    std::fs::copy(&backup, original).map_err(|e| InstallError::BackupFailed {
        path: original.display().to_string(),
        reason: e.to_string(),
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{backup_before_write, backup_path_for, restore, BACKUP_SUFFIX};
    use std::fs;

    #[test]
    fn backup_path_appends_fixed_suffix() {
        let path = std::path::Path::new("/home/user/.claude.json");
        let backup = backup_path_for(path);
        assert_eq!(
            backup,
            std::path::PathBuf::from(format!("/home/user/.claude.json{BACKUP_SUFFIX}"))
        );
    }

    #[test]
    fn backup_before_write_is_noop_for_nonexistent_file() -> Result<(), Box<dyn std::error::Error>>
    {
        let dir = tempfile::tempdir()?;
        let missing = dir.path().join("does-not-exist.json");
        let result = backup_before_write(&missing)?;
        assert!(result.is_none());
        Ok(())
    }

    #[test]
    fn backup_before_write_copies_existing_file() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let original = dir.path().join("config.json");
        fs::write(&original, "{\"original\":true}")?;

        let backup = backup_before_write(&original)?.ok_or("expected a backup path")?;
        assert!(backup.exists());
        assert_eq!(fs::read_to_string(&backup)?, "{\"original\":true}");
        Ok(())
    }

    #[test]
    fn restore_puts_original_bytes_back_after_a_bad_write() -> Result<(), Box<dyn std::error::Error>>
    {
        let dir = tempfile::tempdir()?;
        let original = dir.path().join("config.json");
        fs::write(&original, "{\"good\":true}")?;

        backup_before_write(&original)?;
        fs::write(&original, "{\"corrupted")?; // simulate a bad write

        restore(&original)?;
        assert_eq!(fs::read_to_string(&original)?, "{\"good\":true}");
        Ok(())
    }

    #[test]
    fn restore_without_a_backup_is_a_detected_error() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let original = dir.path().join("config.json");
        fs::write(&original, "{}")?;

        assert!(restore(&original).is_err());
        Ok(())
    }

    #[test]
    fn repeated_backup_overwrites_the_prior_backup_not_a_pile(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let original = dir.path().join("config.json");

        fs::write(&original, "v1")?;
        backup_before_write(&original)?;
        fs::write(&original, "v2")?;
        backup_before_write(&original)?;

        let backup = backup_path_for(&original);
        assert_eq!(fs::read_to_string(&backup)?, "v2");
        Ok(())
    }
}
