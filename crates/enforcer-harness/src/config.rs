//! Harness configuration + storage-root resolution + text redaction.
//!
//! Ported from the `DEFAULT_HARNESS_CONFIG` / `LEGACY_STORAGE_DIR` /
//! `SECRET_REDACTION_PATTERNS` constants in `src/harness.mjs`. Text
//! redaction rides `enforcer_core::redaction::Redactor` (wrapping the raw
//! string in a JSON value) rather than re-inlining the pattern list, per
//! the workpack requirement.

use std::path::{Path, PathBuf};

use enforcer_core::error::Result;
use enforcer_core::redaction::Redactor;
use enforcer_domain::config_types::HarnessConfig;

/// Default `.enforce`-relative storage directory name.
pub const DEFAULT_STORAGE_DIR: &str = ".enforce";

/// Legacy (pre-migration) storage directory name, still read (never
/// written) so existing installs keep their run history. Coordinated with
/// arc-23 install/migration.
pub const LEGACY_STORAGE_DIR: &str = ".ocentra-enforcer";

/// Validate and normalize the configured storage dir. Rejects absolute
/// paths and `..` segments, and normalizes separators to `/`.
pub fn sanitized_storage_dir(config: &HarnessConfig) -> Result<String> {
    sanitize_storage_dir(config.storage_dir.as_str())
}

/// Resolve the authoritative (`.enforce`-rooted by default) storage root.
pub fn storage_root(config: &HarnessConfig, repo_root: &Path) -> Result<PathBuf> {
    Ok(repo_root.join(sanitized_storage_dir(config)?))
}

fn sanitize_storage_dir(value: &str) -> Result<String> {
    let storage_dir = if value.is_empty() {
        DEFAULT_STORAGE_DIR
    } else {
        value
    };
    let looks_absolute = Path::new(storage_dir).is_absolute()
        || storage_dir.starts_with('/')
        || storage_dir.starts_with('\\')
        || has_windows_drive_prefix(storage_dir);
    if looks_absolute || storage_dir.contains("..") {
        return Err(enforcer_core::error::Error::InvalidConfig(format!(
            "invalid harness storageDir: {storage_dir}"
        )));
    }
    Ok(storage_dir.replace('\\', "/"))
}

fn has_windows_drive_prefix(value: &str) -> bool {
    let bytes = value.as_bytes();
    let Some((&first, rest)) = bytes.split_first() else {
        return false;
    };
    let Some(&second) = rest.first() else {
        return false;
    };
    first.is_ascii_alphabetic() && second == b':'
}

/// The legacy storage root path under a repo root (read-only).
pub fn legacy_storage_root(repo_root: &Path) -> PathBuf {
    repo_root.join(LEGACY_STORAGE_DIR)
}

/// Redact secrets from a raw text blob (stdout/stderr/artifact bytes).
///
/// Wraps the text in a JSON string value and runs it through the shared
/// `enforcer_core::redaction::Redactor` value-pattern layer, so the secret
/// pattern list lives in exactly one place in the workspace.
pub fn redact_text(text: &str) -> Result<String> {
    let redactor = Redactor::with_defaults()?;
    let mut value = serde_json::Value::String(text.to_owned());
    redactor.redact(&mut value);
    Ok(value.as_str().unwrap_or_default().to_owned())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{sanitize_storage_dir, storage_root, DEFAULT_STORAGE_DIR};
    use enforcer_core::error::Result;
    use enforcer_domain::config_types::{HarnessConfig, HarnessRetentionDays, HarnessRunLimit};

    #[test]
    fn default_config_matches_legacy_mjs_defaults() -> Result<()> {
        let config = HarnessConfig::default();
        assert_eq!(config.storage_dir.as_str(), DEFAULT_STORAGE_DIR);
        assert_eq!(config.store.as_str(), "ndjson-duckdb");
        assert_eq!(config.max_artifact_bytes.get(), 8000);
        assert_eq!(config.max_runs.map(HarnessRunLimit::get), Some(50));
        assert_eq!(config.max_runs_per_tool.map(HarnessRunLimit::get), Some(20));
        assert_eq!(config.max_failed_runs.map(HarnessRunLimit::get), Some(20));
        assert_eq!(
            config.prune_after_days.map(HarnessRetentionDays::get),
            Some(14)
        );
        assert!(storage_root(&config, Path::new("repo"))?.ends_with(".enforce"));
        Ok(())
    }

    #[test]
    fn sanitize_storage_dir_rejects_absolute_and_traversal() -> Result<()> {
        assert_eq!(
            sanitize_storage_dir("/abs").map_err(|error| error.to_string()),
            Err("invalid core configuration: invalid harness storageDir: /abs".to_owned())
        );
        assert_eq!(
            sanitize_storage_dir("C:/abs").map_err(|error| error.to_string()),
            Err("invalid core configuration: invalid harness storageDir: C:/abs".to_owned())
        );
        assert_eq!(
            sanitize_storage_dir(r"z:\abs").map_err(|error| error.to_string()),
            Err(r"invalid core configuration: invalid harness storageDir: z:\abs".to_owned())
        );
        assert_eq!(
            sanitize_storage_dir("C:relative").map_err(|error| error.to_string()),
            Err("invalid core configuration: invalid harness storageDir: C:relative".to_owned())
        );
        assert_eq!(
            sanitize_storage_dir("../escape").map_err(|error| error.to_string()),
            Err("invalid core configuration: invalid harness storageDir: ../escape".to_owned())
        );
        assert_eq!(sanitize_storage_dir("")?, DEFAULT_STORAGE_DIR);
        assert_eq!(sanitize_storage_dir(r"a\b")?, "a/b");
        Ok(())
    }

    #[test]
    fn redact_text_masks_seeded_secret_and_preserves_clean_text() -> Result<()> {
        let synthetic_access_key = ["AKIA", "IOSFODNN7EXAMPLE"].concat();
        let seeded = super::redact_text(&format!("token: {synthetic_access_key} embedded"))?;
        assert_eq!(seeded, "[REDACTED] embedded");

        let clean = super::redact_text("nothing sensitive here")?;
        assert_eq!(clean, "nothing sensitive here");
        Ok(())
    }
}
