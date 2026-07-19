//! Persist and decode the project security-profile activation record.
//! BOUNDARY-INVARIANT: raw JSON values become canonical domain types before
//! they enter runtime state; malformed, blank, or unsupported values fail closed.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::config_types::{ConfigProfileName, PolicyOwner, PolicyReason};
use enforcer_domain::paths::RelPath;
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};

/// Activation file stored beneath the project enforcement directory.
pub const SECURITY_PROFILE_FILE: &str = "security-profile.json";
/// Canonical money-critical profile name.
pub const MONEY_CRITICAL_PROFILE: &str = "money-critical-security";

/// Validated security-profile activation used inside the application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityProfileActivation {
    pub schema_version: NonZeroU32,
    pub profile_name: ConfigProfileName,
    pub source_spec: RelPath,
    pub owner: PolicyOwner,
    pub reason: PolicyReason,
}

// ROUNDTRIP-TEST: tests/activation.rs verifies JSON encode/decode stability.
/// Raw JSON transport shape for a security-profile activation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityProfileActivationDto {
    pub schema_version: u32,
    pub profile_name: String,
    pub source_spec: String,
    pub owner: String,
    pub reason: String,
}

impl TryFrom<SecurityProfileActivationDto> for SecurityProfileActivation {
    type Error = DecodeError;

    fn try_from(dto: SecurityProfileActivationDto) -> Result<Self, Self::Error> {
        Ok(Self {
            schema_version: NonZeroU32::new(dto.schema_version)
                .ok_or_else(|| DecodeError::new("schemaVersion", "must be greater than zero"))?,
            profile_name: ConfigProfileName::try_new(dto.profile_name)?,
            source_spec: dto.source_spec.parse()?,
            owner: PolicyOwner::try_new(dto.owner)?,
            reason: PolicyReason::try_new(dto.reason)?,
        })
    }
}

impl From<&SecurityProfileActivation> for SecurityProfileActivationDto {
    fn from(activation: &SecurityProfileActivation) -> Self {
        Self {
            schema_version: activation.schema_version.get(),
            profile_name: activation.profile_name.as_str().to_owned(),
            source_spec: activation.source_spec.as_str().to_owned(),
            owner: activation.owner.as_str().to_owned(),
            reason: activation.reason.as_str().to_owned(),
        }
    }
}

/// Resolve the activation file path beneath a project root.
pub fn activation_path(root: &Path) -> PathBuf {
    root.join(".enforce").join(SECURITY_PROFILE_FILE)
}

/// Load and validate a project's activation file when it exists.
pub fn load_project_activation(root: &Path) -> Result<Option<SecurityProfileActivation>, String> {
    let path = activation_path(root);
    if !path.is_file() {
        return Ok(None);
    }
    let source = std::fs::read(&path).map_err(|error| {
        format!(
            "cannot read security activation at {}: {error}",
            path.display()
        )
    })?;
    let activation_dto: SecurityProfileActivationDto =
        serde_json::from_slice(&source).map_err(|error| {
            format!(
                "cannot decode security activation at {}: {error}",
                path.display()
            )
        })?;
    let activation = SecurityProfileActivation::try_from(activation_dto)
        .map_err(|error| format!("invalid security activation at {}: {error}", path.display()))?;
    validate_activation(&activation)?;
    Ok(Some(activation))
}

/// Validate and persist a project's activation file.
pub fn write_project_activation(
    root: &Path,
    activation: &SecurityProfileActivation,
) -> Result<(), String> {
    validate_activation(activation)?;
    let path = activation_path(root);
    let parent = path
        .parent()
        .ok_or("security activation path has no parent")?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create security activation directory: {error}"))?;
    let activation_dto = SecurityProfileActivationDto::from(activation);
    let content = serde_json::to_vec_pretty(&activation_dto)
        .map_err(|error| format!("cannot encode security activation: {error}"))?;
    std::fs::write(&path, content).map_err(|error| {
        format!(
            "cannot write security activation at {}: {error}",
            path.display()
        )
    })
}

fn validate_activation(activation: &SecurityProfileActivation) -> Result<(), String> {
    if activation.schema_version.get() != 1 {
        return Err("security activation schemaVersion must be 1".to_owned());
    }
    if activation.profile_name.as_str() != MONEY_CRITICAL_PROFILE {
        return Err(format!(
            "unsupported security profile: {}",
            activation.profile_name.as_str()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{SecurityProfileActivation, SecurityProfileActivationDto};

    #[test]
    fn invalid_transport_values_are_rejected_before_domain_entry(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let invalid = SecurityProfileActivationDto {
            schema_version: 0,
            profile_name: String::new(),
            source_spec: String::new(),
            owner: String::new(),
            reason: String::new(),
        };

        let error = SecurityProfileActivation::try_from(invalid)
            .err()
            .ok_or("zero schema version must be rejected")?;
        assert_eq!(error.path, "schemaVersion");
        assert_eq!(error.reason, "must be greater than zero");
        Ok(())
    }
}
