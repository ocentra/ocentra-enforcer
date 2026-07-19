//! Persisted baseline DTOs.

use enforcer_domain::findings::FindingLine;
use enforcer_domain::hashes::Sha256;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::telemetry_types::RecordSchemaVersion;

use crate::rules::baseline_ratchet::BaselineLocation;

/// One persisted baseline occurrence key.
/// ROUNDTRIP-TEST: `tests/baseline_ratchet.rs::clean_baseline_write_round_trips_via_persisted_record`
/// proves that this record survives the persisted wire cycle.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaselineEntryDto {
    /// Rule that fired.
    pub rule_id: RuleId,
    /// File the violation was recorded against.
    pub file: RelPath,
    /// Source line at which the violation was recorded.
    pub line: FindingLine,
}

/// Versioned, integrity-hashed persisted baseline record.
/// ROUNDTRIP-TEST: `tests/baseline_ratchet.rs::clean_baseline_write_round_trips_via_persisted_record`
/// verifies the version, entries, and integrity digest together.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaselineRecordDto {
    /// Schema version of this record.
    pub version: RecordSchemaVersion,
    /// Sorted persisted occurrence keys.
    pub entries: Vec<BaselineEntryDto>,
    /// Integrity digest over the canonical entry payload.
    pub integrity: Sha256,
}

impl From<&BaselineLocation> for BaselineEntryDto {
    fn from(location: &BaselineLocation) -> Self {
        Self {
            rule_id: location.rule_id.clone(),
            file: location.file.clone(),
            line: location.line,
        }
    }
}

/// NEGATIVE-CONVERSION-TEST: `tests/baseline_ratchet.rs::tampered_baseline_file_fails_to_load`
/// rejects a bad persisted record before it can produce a [`BaselineLocation`].
impl From<BaselineEntryDto> for BaselineLocation {
    fn from(entry: BaselineEntryDto) -> Self {
        Self {
            rule_id: entry.rule_id,
            file: entry.file,
            line: entry.line,
        }
    }
}
