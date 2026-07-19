//! Serialized coverage DTOs.

use enforcer_domain::paths::RelPath;
use enforcer_domain::scan_types::{ScanTargetCount, SkipReason};

use crate::coverage::{Coverage, SkipRecord};

/// One skipped target in the coverage report wire form.
/// ROUNDTRIP-TEST: `coverage::tests::coverage_roundtrip_through_json` proves
/// a decoded record converts back to the canonical scan-domain value.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct SkipRecordDto {
    /// The file/target that was skipped.
    pub file: RelPath,
    /// Why the target was skipped. Never empty.
    pub reason: SkipReason,
}

/// Aggregated ran/skipped accounting in the serialized report wire form.
/// ROUNDTRIP-TEST: `coverage::tests::coverage_roundtrip_through_json` proves
/// the report wire shape preserves the complete canonical coverage value.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct CoverageDto {
    /// Number of targets that ran at least the dispatch.
    pub ran_count: ScanTargetCount,
    /// Number of targets skipped with an explicit reason.
    pub skipped_count: ScanTargetCount,
    /// Skip records retained so partial or hollow scans remain visible.
    pub skips: Vec<SkipRecordDto>,
}

/// Decode one coverage DTO at the JSON boundary.
pub fn decode_coverage_json(payload: &str) -> Result<CoverageDto, serde_json::Error> {
    serde_json::from_str(payload)
}

impl SkipRecordDto {
    /// Convert an already-decoded report DTO into scan-domain accounting.
    pub fn into_domain(self) -> SkipRecord {
        SkipRecord {
            file: self.file,
            reason: self.reason,
        }
    }
}

impl CoverageDto {
    /// Convert a report DTO into canonical scan-domain coverage accounting.
    pub fn into_domain(self) -> Coverage {
        Coverage::from_parts(
            self.ran_count,
            self.skipped_count,
            self.skips
                .into_iter()
                .map(SkipRecordDto::into_domain)
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{decode_coverage_json, CoverageDto, SkipRecordDto};
    use enforcer_domain::paths::RelPath;
    use enforcer_domain::scan_types::{ScanTargetCount, SkipReason};

    #[test]
    fn coverage_dto_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let skip = SkipRecordDto {
            file: "src/lib.rs".parse::<RelPath>()?,
            reason: SkipReason::try_new("unmatched extension".to_owned())?,
        };
        let skip_wire = serde_json::to_string(&skip)?;
        let restored_skip: SkipRecordDto = serde_json::from_str(&skip_wire)?;
        assert_eq!(restored_skip, skip);

        let dto = CoverageDto {
            ran_count: ScanTargetCount::from_count(1),
            skipped_count: ScanTargetCount::from_count(1),
            skips: vec![skip],
        };
        let wire = serde_json::to_string(&dto)?;
        let restored: CoverageDto = serde_json::from_str(&wire)?;
        assert_eq!(restored, dto);
        assert_eq!(decode_coverage_json(&wire)?, dto);
        Ok(())
    }
}
