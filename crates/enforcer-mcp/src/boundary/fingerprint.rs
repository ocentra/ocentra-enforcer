//! MCP fingerprint JSON boundary.
//!
//! BOUNDARY-INVARIANT: raw MCP fingerprint JSON is decoded here and
//! immediately converted into canonical `enforcer-domain` values.
//! boundaryOwnerNote: enforcer-mcp owns MCP artifact fingerprint wire conversion.
//!
//! BOUNDARY-INVARIANT: raw JSON enters and leaves only through these DTOs;
//! canonical fingerprint values remain in enforcer-domain.
//! boundaryOwnerNote: enforcer-mcp owns MCP artifact fingerprint wire conversion.

use enforcer_domain::mcp_types::{ArtifactSlot, ChangedArtifact, McpFingerprint, PackageVersion};
use enforcer_domain::{boundary::decode_error::DecodeError, hashes::Sha256};
use serde::{Deserialize, Serialize};

use crate::boundary::fingerprint_artifact::ArtifactEntryDto;

// ROUNDTRIP-TEST: crates/enforcer-mcp/tests/wire_roundtrip.rs covers every
// externally deserialized fingerprint DTO declared in this module.

/// JSON transport representation of an MCP fingerprint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpFingerprintDto {
    pub digest: String,
    pub package_version: String,
    pub binary: ArtifactEntryDto,
    pub ruleset: Option<ArtifactEntryDto>,
}

/// JSON transport representation of the named artifact slot.
#[doc = "SERDE-TAG-JUSTIFICATION: established compact string artifact-slot wire contract."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ArtifactSlotDto {
    Binary,
    Ruleset,
}

/// JSON transport representation of a changed artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangedArtifactDto {
    pub slot: ArtifactSlotDto,
    pub startup: Option<ArtifactEntryDto>,
    pub current: Option<ArtifactEntryDto>,
}

/// Failure while crossing the MCP JSON fingerprint boundary.
#[derive(Debug, thiserror::Error)]
pub enum FingerprintWireError {
    #[error("invalid fingerprint JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid fingerprint values: {0}")]
    Decode(#[from] DecodeError),
}

impl From<McpFingerprint> for McpFingerprintDto {
    fn from(value: McpFingerprint) -> Self {
        Self {
            digest: value.digest.as_str().to_owned(),
            package_version: value.package_version.as_str().to_owned(),
            binary: value.binary.into(),
            ruleset: value.ruleset.map(Into::into),
        }
    }
}

impl TryFrom<McpFingerprintDto> for McpFingerprint {
    type Error = DecodeError;

    fn try_from(value: McpFingerprintDto) -> Result<Self, Self::Error> {
        Ok(Self {
            digest: Sha256::try_from(value.digest)
                .map_err(|error| DecodeError::new("fingerprint.digest", error.reason))?,
            package_version: PackageVersion::try_new(&value.package_version)
                .map_err(|error| DecodeError::new("fingerprint.packageVersion", error.reason))?,
            binary: value.binary.try_into()?,
            ruleset: value.ruleset.map(TryInto::try_into).transpose()?,
        })
    }
}

impl From<ArtifactSlot> for ArtifactSlotDto {
    fn from(value: ArtifactSlot) -> Self {
        match value {
            ArtifactSlot::Binary => Self::Binary,
            ArtifactSlot::Ruleset => Self::Ruleset,
        }
    }
}

impl From<ArtifactSlotDto> for ArtifactSlot {
    fn from(value: ArtifactSlotDto) -> Self {
        match value {
            ArtifactSlotDto::Binary => Self::Binary,
            ArtifactSlotDto::Ruleset => Self::Ruleset,
        }
    }
}

impl From<ChangedArtifact> for ChangedArtifactDto {
    fn from(value: ChangedArtifact) -> Self {
        Self {
            slot: value.slot.into(),
            startup: value.startup.map(Into::into),
            current: value.current.map(Into::into),
        }
    }
}

impl TryFrom<ChangedArtifactDto> for ChangedArtifact {
    type Error = DecodeError;

    fn try_from(value: ChangedArtifactDto) -> Result<Self, Self::Error> {
        Ok(Self {
            slot: value.slot.into(),
            startup: value.startup.map(TryInto::try_into).transpose()?,
            current: value.current.map(TryInto::try_into).transpose()?,
        })
    }
}

/// Encode a canonical fingerprint for MCP JSON egress.
pub fn encode_fingerprint_json(value: &McpFingerprint) -> Result<String, FingerprintWireError> {
    Ok(serde_json::to_string(&McpFingerprintDto::from(
        value.clone(),
    ))?)
}

/// Decode an MCP JSON fingerprint at ingress into pure canonical values.
pub fn decode_fingerprint_json(value: &str) -> Result<McpFingerprint, FingerprintWireError> {
    let wire: McpFingerprintDto = serde_json::from_str(value)?;
    Ok(wire.try_into()?)
}

#[cfg(test)]
mod tests {
    use super::{decode_fingerprint_json, FingerprintWireError, McpFingerprintDto};
    use crate::boundary::fingerprint_artifact::{ArtifactEntryDto, ArtifactStateDto};

    #[test]
    fn fingerprint_dto_round_trips_at_the_wire_boundary() -> Result<(), serde_json::Error> {
        let dto = McpFingerprintDto {
            digest: "0".repeat(64),
            package_version: "1.0.0".to_owned(),
            binary: ArtifactEntryDto {
                path: "target/enforcer".to_owned(),
                state: ArtifactStateDto::Missing,
            },
            ruleset: None,
        };
        let encoded = serde_json::to_string(&dto)?;
        let decoded: McpFingerprintDto = serde_json::from_str(&encoded)?;
        assert_eq!(decoded, dto);
        Ok(())
    }

    #[test]
    fn fingerprint_json_rejects_an_invalid_domain_digest() {
        let invalid = r#"{"digest":"not-a-digest","packageVersion":"1.0.0","binary":{"path":"target/enforcer","state":{"kind":"missing"}}}"#;
        assert!(matches!(
            decode_fingerprint_json(invalid),
            Err(FingerprintWireError::Decode(error)) if error.path == "fingerprint.digest"
        ));
    }
}
