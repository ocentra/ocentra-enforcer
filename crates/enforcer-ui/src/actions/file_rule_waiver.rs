//! Typed persistence for project-local file-and-rule waivers.

use std::path::{Path, PathBuf};

use enforcer_domain::ids::RuleId;
use enforcer_rules::registry::RuleRegistry;
use enforcer_rules::waiver::{ExpiryPolicy, Waiver, WaiverDate, WaiverLoadError, WaiverRegistry};

/// Relative location of the project-local waiver registry.
pub const PROJECT_WAIVER_REGISTRY_RELATIVE_PATH: &str = ".enforce/waivers.json";

/// A named exception for one known rule and one exact project-relative path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRuleWaiverRequest {
    /// Exact project-relative path affected by the finding.
    pub path: String,
    /// Known rule to waive for this path.
    pub rule_id: RuleId,
    /// Accountable human or team.
    pub owner: String,
    /// Concrete reason the finding is temporarily accepted.
    pub reason: String,
    /// Optional inclusive UTC expiry date.
    pub expires: Option<WaiverDate>,
}

/// Failure while persisting a project-local file-rule waiver.
#[derive(Debug, thiserror::Error)]
pub enum FileRuleWaiverWriteError {
    /// The requested project root is not a directory.
    #[error("project root `{path}` is not a directory")]
    InvalidProjectRoot { path: String },
    /// The existing or proposed strict waiver registry was invalid.
    #[error(transparent)]
    Waiver(#[from] WaiverLoadError),
    /// The candidate registry could not be encoded as JSON.
    #[error("failed to serialize waiver registry: {reason}")]
    Serialize { reason: String },
    /// A filesystem operation required for the atomic write failed.
    #[error("failed to write waiver registry `{path}`: {reason}")]
    Io { path: String, reason: String },
}

/// Return the canonical project-local waiver registry location.
pub fn project_waiver_registry_path(project_root: &Path) -> PathBuf {
    project_root.join(PROJECT_WAIVER_REGISTRY_RELATIVE_PATH)
}

/// Upsert one file-and-rule waiver into `<project-root>/.enforce/waivers.json`.
///
/// The full candidate registry validates with [`ExpiryPolicy::RejectExpired`]
/// before this function creates `.enforce` or replaces the registry file.
/// Repeating the same `(path, rule_id)` replaces that row and yields stable
/// bytes after the first write.
pub fn upsert_file_rule_waiver(
    project_root: &Path,
    rules: &RuleRegistry,
    today: WaiverDate,
    request: &FileRuleWaiverRequest,
) -> Result<WaiverRegistry, FileRuleWaiverWriteError> {
    if !project_root.is_dir() {
        return Err(FileRuleWaiverWriteError::InvalidProjectRoot {
            path: project_root.display().to_string(),
        });
    }

    let registry_path = project_waiver_registry_path(project_root);
    let mut registry = if registry_path.exists() {
        WaiverRegistry::load_file(&registry_path, rules, today, ExpiryPolicy::RejectExpired)?
    } else {
        WaiverRegistry::default()
    };

    let existing_path = registry
        .matching(&request.path, &request.rule_id, today)
        .map(|waiver| waiver.path.clone());
    registry.waivers.retain(|waiver| {
        existing_path
            .as_ref()
            .is_none_or(|path| waiver.path != *path || waiver.rule_id != request.rule_id)
    });
    registry.waivers.push(Waiver {
        path: request.path.clone(),
        rule_id: request.rule_id.clone(),
        owner: request.owner.clone(),
        reason: request.reason.clone(),
        expires: request.expires,
    });
    registry.validate(rules, today, ExpiryPolicy::RejectExpired)?;

    let mut serialized = serde_json::to_string_pretty(&registry).map_err(|error| {
        FileRuleWaiverWriteError::Serialize {
            reason: error.to_string(),
        }
    })?;
    serialized.push('\n');

    let parent = registry_path
        .parent()
        .ok_or_else(|| FileRuleWaiverWriteError::Io {
            path: registry_path.display().to_string(),
            reason: "waiver registry path has no parent directory".to_owned(),
        })?;
    std::fs::create_dir_all(parent).map_err(|error| FileRuleWaiverWriteError::Io {
        path: parent.display().to_string(),
        reason: error.to_string(),
    })?;
    write_atomic(&registry_path, serialized.as_bytes())?;

    Ok(registry)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), FileRuleWaiverWriteError> {
    let unique = format!(
        "{}.{}.tmp",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default()
    );
    let temporary_path = path.with_extension(unique);
    std::fs::write(&temporary_path, bytes).map_err(|error| FileRuleWaiverWriteError::Io {
        path: temporary_path.display().to_string(),
        reason: error.to_string(),
    })?;
    std::fs::rename(&temporary_path, path).map_err(|error| FileRuleWaiverWriteError::Io {
        path: path.display().to_string(),
        reason: error.to_string(),
    })
}
