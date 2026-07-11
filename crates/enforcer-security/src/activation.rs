use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const SECURITY_PROFILE_FILE: &str = "security-profile.json";
pub const MONEY_CRITICAL_PROFILE: &str = "money-critical-security";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityProfileActivation {
    pub schema_version: u32,
    pub profile_name: String,
    pub source_spec: String,
    pub owner: String,
    pub reason: String,
}

pub fn activation_path(root: &Path) -> PathBuf {
    root.join(".enforce").join(SECURITY_PROFILE_FILE)
}

pub fn load_project_activation(root: &Path) -> Result<Option<SecurityProfileActivation>, String> {
    let path = activation_path(root);
    if !path.is_file() {
        return Ok(None);
    }
    let source = std::fs::read(&path)
        .map_err(|error| format!("cannot read security activation at {}: {error}", path.display()))?;
    let activation = serde_json::from_slice(&source)
        .map_err(|error| format!("cannot decode security activation at {}: {error}", path.display()))?;
    validate_activation(&activation)?;
    Ok(Some(activation))
}

pub fn write_project_activation(root: &Path, activation: &SecurityProfileActivation) -> Result<(), String> {
    validate_activation(activation)?;
    let path = activation_path(root);
    let parent = path.parent().ok_or("security activation path has no parent")?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create security activation directory: {error}"))?;
    let content = serde_json::to_vec_pretty(activation)
        .map_err(|error| format!("cannot encode security activation: {error}"))?;
    std::fs::write(&path, content)
        .map_err(|error| format!("cannot write security activation at {}: {error}", path.display()))
}

fn validate_activation(activation: &SecurityProfileActivation) -> Result<(), String> {
    if activation.schema_version != 1 {
        return Err("security activation schemaVersion must be 1".to_owned());
    }
    if activation.profile_name != MONEY_CRITICAL_PROFILE {
        return Err(format!("unsupported security profile: {}", activation.profile_name));
    }
    for (field, value) in [("sourceSpec", &activation.source_spec), ("owner", &activation.owner), ("reason", &activation.reason)] {
        if value.trim().is_empty() {
            return Err(format!("security activation {field} must not be empty"));
        }
    }
    Ok(())
}
