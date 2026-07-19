//! MCP staleness-report JSON boundary.
//!
//! BOUNDARY-INVARIANT: raw report digests are validated and converted into
//! canonical digest values before leaving this module.
//! NEGATIVE-TEST: malformed report digests are rejected during conversion.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::hashes::Sha256;
use enforcer_domain::mcp_types::{Staleness, StalenessReport};
use serde::{Deserialize, Serialize};

use crate::boundary::fingerprint::{ChangedArtifactDto, FingerprintWireError};

/// JSON transport representation of the freshness verdict.
#[doc = "SERDE-TAG-JUSTIFICATION: established compact string freshness wire contract."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StalenessDto {
    Fresh,
    Stale,
}

/// JSON transport representation of an MCP fingerprint comparison.
// ROUNDTRIP-TEST: the boundary test module below serializes and deserializes this DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StalenessReportDto {
    pub verdict: StalenessDto,
    pub startup_digest: String,
    pub current_digest: String,
    pub changed: Vec<ChangedArtifactDto>,
}

impl From<Staleness> for StalenessDto {
    fn from(value: Staleness) -> Self {
        match value {
            Staleness::Fresh => Self::Fresh,
            Staleness::Stale => Self::Stale,
        }
    }
}

impl From<StalenessDto> for Staleness {
    fn from(value: StalenessDto) -> Self {
        match value {
            StalenessDto::Fresh => Self::Fresh,
            StalenessDto::Stale => Self::Stale,
        }
    }
}

impl From<StalenessReport> for StalenessReportDto {
    fn from(value: StalenessReport) -> Self {
        Self {
            verdict: value.verdict.into(),
            startup_digest: value.startup_digest.as_str().to_owned(),
            current_digest: value.current_digest.as_str().to_owned(),
            changed: value.changed.into_iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<StalenessReportDto> for StalenessReport {
    type Error = DecodeError;

    fn try_from(value: StalenessReportDto) -> Result<Self, Self::Error> {
        Ok(Self {
            verdict: value.verdict.into(),
            startup_digest: Sha256::try_from(value.startup_digest)
                .map_err(|error| DecodeError::new("staleness.startupDigest", error.reason))?,
            current_digest: Sha256::try_from(value.current_digest)
                .map_err(|error| DecodeError::new("staleness.currentDigest", error.reason))?,
            changed: value
                .changed
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

/// Encode a canonical freshness report for MCP JSON egress.
pub fn encode_staleness_report_json(
    value: &StalenessReport,
) -> Result<String, FingerprintWireError> {
    Ok(serde_json::to_string(&StalenessReportDto::from(
        value.clone(),
    ))?)
}

/// Decode an MCP JSON freshness report into pure canonical values.
pub fn decode_staleness_report_json(value: &str) -> Result<StalenessReport, FingerprintWireError> {
    let wire: StalenessReportDto = serde_json::from_str(value)?;
    Ok(wire.try_into()?)
}

#[cfg(test)]
mod tests {
    use super::{StalenessDto, StalenessReportDto};

    #[test]
    fn staleness_report_dto_round_trips_through_serde() -> Result<(), serde_json::Error> {
        let dto = StalenessReportDto {
            verdict: StalenessDto::Stale,
            startup_digest: "1".repeat(64),
            current_digest: "2".repeat(64),
            changed: Vec::new(),
        };
        let encoded = serde_json::to_string(&dto)?;
        let decoded: StalenessReportDto = serde_json::from_str(&encoded)?;
        assert_eq!(decoded, dto);
        Ok(())
    }

    #[test]
    fn staleness_report_rejects_an_invalid_startup_digest() {
        let dto = StalenessReportDto {
            verdict: StalenessDto::Stale,
            startup_digest: "invalid".to_owned(),
            current_digest: "0".repeat(64),
            changed: Vec::new(),
        };
        let result = enforcer_domain::mcp_types::StalenessReport::try_from(dto);
        assert!(matches!(
            result,
            Err(error) if error.path == "staleness.startupDigest"
        ));
    }
}
