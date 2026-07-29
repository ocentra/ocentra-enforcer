//! External proof for the filesystem backup boundary.

use enforcer_install::backup::{backup_before_write, backup_path_for, restore, BACKUP_SUFFIX};
use enforcer_install::error::InstallError;
use std::fs;

#[test]
fn backup_boundary_preserves_original_bytes_and_reports_missing_backup(
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let original = directory.path().join("config.json");
    fs::write(&original, "{\"version\":1}")?;

    let backup = backup_before_write(&original)?.ok_or("existing file must produce a backup")?;
    assert_eq!(backup, backup_path_for(&original));
    assert_eq!(
        backup
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_owned),
        Some(format!("config.json{BACKUP_SUFFIX}"))
    );

    fs::write(&original, "{\"corrupted\":true}")?;
    restore(&original)?;
    assert_eq!(fs::read_to_string(&original)?, "{\"version\":1}");

    let absent = directory.path().join("absent.json");
    let failure = restore(&absent);
    assert_eq!(
        failure,
        Err(InstallError::BackupFailed {
            path: absent.display().to_string(),
            reason: format!(
                "no backup found at `{}`",
                backup_path_for(&absent).display()
            ),
        })
    );
    Ok(())
}
