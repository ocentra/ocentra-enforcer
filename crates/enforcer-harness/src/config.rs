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

/// Default `.enforce`-relative storage directory name.
pub const DEFAULT_STORAGE_DIR: &str = ".enforce";

/// Legacy (pre-migration) storage directory name, still read (never
/// written) so existing installs keep their run history. Coordinated with
/// arc-23 install/migration.
pub const LEGACY_STORAGE_DIR: &str = ".ocentra-enforcer";

/// Harness run-store configuration (retention knobs + storage layout).
/// Mirrors `DEFAULT_HARNESS_CONFIG` in `src/harness.mjs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessConfig {
    /// Storage directory name, relative to the repo root (default
    /// `.enforce`).
    pub storage_dir: String,
    /// Store contract name (`ndjson-duckdb`; NDJSON authoritative).
    pub store: String,
    /// Byte cap applied when reading an artifact.
    pub max_artifact_bytes: usize,
    /// Maximum total runs kept (`None` = unlimited).
    pub max_runs: Option<usize>,
    /// Maximum runs kept per tool (`None` = unlimited).
    pub max_runs_per_tool: Option<usize>,
    /// Maximum failed runs kept regardless of age/count (`None` =
    /// unlimited).
    pub max_failed_runs: Option<usize>,
    /// Runs older than this many days are pruned (`None` = unlimited).
    pub prune_after_days: Option<i64>,
}

impl Default for HarnessConfig {
    fn default() -> Self {
        Self {
            storage_dir: DEFAULT_STORAGE_DIR.to_owned(),
            store: "ndjson-duckdb".to_owned(),
            max_artifact_bytes: 8000,
            max_runs: Some(50),
            max_runs_per_tool: Some(20),
            max_failed_runs: Some(20),
            prune_after_days: Some(14),
        }
    }
}

impl HarnessConfig {
    /// Validate and normalize the configured storage dir. Rejects absolute
    /// paths and `..` segments, and normalizes separators to `/` — mirrors
    /// `sanitizeStorageDir` in `src/harness.mjs`.
    pub fn sanitized_storage_dir(&self) -> Result<String> {
        sanitize_storage_dir(&self.storage_dir)
    }

    /// Resolve the authoritative (`.enforce`-rooted by default) storage
    /// root under a repo root.
    pub fn storage_root(&self, repo_root: &Path) -> Result<PathBuf> {
        Ok(repo_root.join(self.sanitized_storage_dir()?))
    }
}

fn sanitize_storage_dir(value: &str) -> Result<String> {
    let storage_dir = if value.is_empty() {
        DEFAULT_STORAGE_DIR
    } else {
        value
    };
    let looks_absolute = Path::new(storage_dir).is_absolute()
        || storage_dir.starts_with('/')
        || storage_dir.starts_with('\\');
    if looks_absolute || storage_dir.contains("..") {
        return Err(enforcer_core::error::Error::InvalidConfig(format!(
            "invalid harness storageDir: {storage_dir}"
        )));
    }
    Ok(storage_dir.replace('\\', "/"))
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
    use super::{sanitize_storage_dir, HarnessConfig, DEFAULT_STORAGE_DIR};
    use enforcer_core::error::Result;

    #[test]
    fn default_config_matches_legacy_mjs_defaults() {
        let config = HarnessConfig::default();
        assert_eq!(config.storage_dir, DEFAULT_STORAGE_DIR);
        assert_eq!(config.store, "ndjson-duckdb");
        assert_eq!(config.max_artifact_bytes, 8000);
        assert_eq!(config.max_runs, Some(50));
        assert_eq!(config.max_runs_per_tool, Some(20));
        assert_eq!(config.max_failed_runs, Some(20));
        assert_eq!(config.prune_after_days, Some(14));
    }

    #[test]
    fn sanitize_storage_dir_rejects_absolute_and_traversal() -> Result<()> {
        assert!(sanitize_storage_dir("/abs").is_err());
        assert!(sanitize_storage_dir("C:/abs").is_err());
        assert!(sanitize_storage_dir("../escape").is_err());
        assert_eq!(sanitize_storage_dir("")?, DEFAULT_STORAGE_DIR);
        assert_eq!(sanitize_storage_dir(r"a\b")?, "a/b");
        Ok(())
    }

    #[test]
    fn redact_text_masks_seeded_secret_and_preserves_clean_text() -> Result<()> {
        let synthetic_access_key = ["AKIA", "IOSFODNN7EXAMPLE"].concat();
        let seeded = super::redact_text(&format!("token: {synthetic_access_key} embedded"))?;
        assert!(!seeded.contains(&synthetic_access_key));
        assert!(seeded.contains(enforcer_core::redaction::REDACTED));

        let clean = super::redact_text("nothing sensitive here")?;
        assert_eq!(clean, "nothing sensitive here");
        Ok(())
    }
}
