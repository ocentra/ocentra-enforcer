//! MCP artifact-observation JSON boundary.
//!
//! BOUNDARY-INVARIANT: raw artifact paths, digests, and byte lengths are
//! converted into canonical artifact values before leaving this module.
//! NEGATIVE-TEST: an empty artifact path and an invalid digest are rejected.
//! boundaryOwnerNote: enforcer-mcp owns artifact fingerprint wire conversion.

use std::path::Path;

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::hashes::Sha256;
use enforcer_domain::mcp_types::{ArtifactEntry, ArtifactPath, ArtifactState, ByteCount};
use serde::{Deserialize, Serialize};

/// JSON transport representation of one artifact state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ArtifactStateDto {
    Present { sha256: String, byte_length: u64 },
    Missing,
}

/// JSON transport representation of an artifact observation.
// ROUNDTRIP-TEST: the boundary test module below serializes and deserializes this DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactEntryDto {
    pub path: String,
    pub state: ArtifactStateDto,
}

impl From<ArtifactState> for ArtifactStateDto {
    fn from(value: ArtifactState) -> Self {
        match value {
            ArtifactState::Present {
                sha256,
                byte_length,
            } => Self::Present {
                sha256: sha256.as_str().to_owned(),
                byte_length: u64::from(byte_length),
            },
            ArtifactState::Missing => Self::Missing,
        }
    }
}

impl TryFrom<ArtifactStateDto> for ArtifactState {
    type Error = DecodeError;

    fn try_from(value: ArtifactStateDto) -> Result<Self, Self::Error> {
        match value {
            ArtifactStateDto::Present {
                sha256,
                byte_length,
            } => Ok(Self::Present {
                sha256: Sha256::try_from(sha256)
                    .map_err(|error| DecodeError::new("artifact.state.sha256", error.reason))?,
                byte_length: std::num::NonZeroU64::new(byte_length)
                    .map_or(ByteCount::ZERO, ByteCount::try_new),
            }),
            ArtifactStateDto::Missing => Ok(Self::Missing),
        }
    }
}

impl From<ArtifactEntry> for ArtifactEntryDto {
    fn from(value: ArtifactEntry) -> Self {
        Self {
            path: value.path.as_str().to_owned(),
            state: value.state.into(),
        }
    }
}

impl TryFrom<ArtifactEntryDto> for ArtifactEntry {
    type Error = DecodeError;

    fn try_from(value: ArtifactEntryDto) -> Result<Self, Self::Error> {
        if value.path.trim().is_empty() {
            return Err(DecodeError::new("artifact.path", "must not be empty"));
        }
        Ok(Self {
            path: ArtifactPath::from_path(Path::new(&value.path)),
            state: value.state.try_into()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{ArtifactEntryDto, ArtifactStateDto};

    #[test]
    fn artifact_entry_dto_round_trips_through_serde() -> Result<(), serde_json::Error> {
        let dto = ArtifactEntryDto {
            path: "target/enforcer".to_owned(),
            state: ArtifactStateDto::Present {
                sha256: "0".repeat(64),
                byte_length: 19,
            },
        };
        let encoded = serde_json::to_string(&dto)?;
        let decoded: ArtifactEntryDto = serde_json::from_str(&encoded)?;
        assert_eq!(decoded, dto);
        Ok(())
    }

    #[test]
    fn artifact_entry_rejects_an_empty_path() {
        let dto = ArtifactEntryDto {
            path: " ".to_owned(),
            state: ArtifactStateDto::Missing,
        };
        let result = enforcer_domain::mcp_types::ArtifactEntry::try_from(dto);
        assert!(matches!(result, Err(error) if error.path == "artifact.path"));
    }
}
