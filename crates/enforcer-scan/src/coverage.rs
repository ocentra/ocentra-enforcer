//! Scan-coverage accounting — aggregates per-target [`crate::outcome::
//! Outcome`]s into ran/skipped counts and enforces the anti-silent-skip
//! invariant: a scan that ran zero checks is a hard failure, never a
//! clean pass.
//!
//! A partial scan (some files skipped for a legitimate reason — unmatched
//! extension, missing tool, unreadable file, empty selection) is fine and
//! stays visible in the report. A scan that ran *nothing* — the hollow
//! self-scan, green because it checked nothing — is not a partial scan;
//! it is a failure this module surfaces explicitly rather than letting an
//! empty findings list masquerade as "all clear".

use enforcer_core::error::DecodeError;

use crate::outcome::{Outcome, SkipReason};
use enforcer_domain::paths::RelPath;

/// One target's recorded outcome, keyed by the repo-relative path it
/// applies to. This is the unit [`Coverage::from_outcomes`] folds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetOutcome {
    /// The file/target this outcome describes.
    pub file: RelPath,
    /// What happened to it.
    pub outcome: Outcome,
}

/// One skip, surfaced in a report so a partial (or hollow) scan is
/// visible rather than silently absorbed into an empty findings list.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct SkipRecord {
    /// The file/target that was skipped.
    pub file: RelPath,
    /// Why. Never empty — see [`crate::outcome::SkipReason`].
    pub reason: SkipReason,
}

/// Aggregated ran/skipped accounting for one scan run.
///
/// Construct via [`Coverage::from_outcomes`]; the two counters and the
/// skip list are always internally consistent with the outcomes folded
/// in (no field is independently mutable after construction).
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct Coverage {
    /// Number of targets that actually ran at least the dispatch (i.e.
    /// produced `Outcome::Ran`).
    pub ran_count: usize,
    /// Number of targets that were skipped, with a reason.
    pub skipped_count: usize,
    /// The skip records themselves (file + non-empty reason), so a
    /// hollow or partial scan is visible in the serialized report, not
    /// just a bare count.
    pub skips: Vec<SkipRecord>,
}

impl Coverage {
    /// Fold a stream of per-target outcomes into aggregated coverage.
    pub fn from_outcomes(outcomes: impl IntoIterator<Item = TargetOutcome>) -> Self {
        let mut coverage = Coverage::default();
        for TargetOutcome { file, outcome } in outcomes {
            match outcome {
                Outcome::Ran { .. } => coverage.ran_count += 1,
                Outcome::Skipped { reason } => {
                    coverage.skipped_count += 1;
                    coverage.skips.push(SkipRecord { file, reason });
                }
            }
        }
        coverage
    }

    /// Total targets accounted for (ran + skipped). Every candidate
    /// handed to the engine must appear in this total exactly once — a
    /// target that is neither ran nor skipped is the silent-skip bug
    /// this module exists to prevent, and by construction cannot arise
    /// from [`Coverage::from_outcomes`] since every [`Outcome`] variant
    /// is folded into one counter or the other.
    pub fn total(&self) -> usize {
        self.ran_count + self.skipped_count
    }

    /// Anti-silent-skip gate: a scan that ran zero checks is never a
    /// clean pass, even if it also skipped nothing (e.g. an empty
    /// selection) or skipped everything. Callers (the CLI, CI wiring)
    /// must call this before reporting success.
    ///
    /// # Errors
    /// Returns [`DecodeError`] describing the hollow-scan condition when
    /// `ran_count` is zero.
    pub fn require_nonzero_ran(&self) -> Result<(), DecodeError> {
        if self.ran_count == 0 {
            Err(DecodeError::new(
                "coverage.ran_count",
                format!(
                    "scan ran zero checks (skipped {}) — a scan that checks nothing is not a \
                     clean pass; this is a hard failure (anti-silent-skip)",
                    self.skipped_count
                ),
            ))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Coverage, TargetOutcome};
    use crate::outcome::Outcome;
    use enforcer_domain::paths::RelPath;

    fn target(path: &str, outcome: Outcome) -> Result<TargetOutcome, Box<dyn std::error::Error>> {
        Ok(TargetOutcome {
            file: path.parse::<RelPath>()?,
            outcome,
        })
    }

    #[test]
    fn empty_outcomes_yield_zero_and_fail_gate() {
        let coverage = Coverage::from_outcomes(std::iter::empty());
        assert_eq!(coverage.total(), 0);
        assert!(coverage.require_nonzero_ran().is_err());
    }

    #[test]
    fn all_skipped_fails_the_zero_ran_gate() -> Result<(), Box<dyn std::error::Error>> {
        let outcomes = vec![
            target("src/a.rs", Outcome::skipped("unmatched extension")?)?,
            target("src/b.bin", Outcome::skipped("missing tool")?)?,
        ];
        let coverage = Coverage::from_outcomes(outcomes);
        assert_eq!(coverage.ran_count, 0);
        assert_eq!(coverage.skipped_count, 2);
        assert_eq!(coverage.total(), 2);
        assert!(
            coverage.require_nonzero_ran().is_err(),
            "all-skipped scan must hard-fail: this is the hollow self-scan"
        );
        Ok(())
    }

    #[test]
    fn mixed_ran_and_skipped_passes_gate_and_surfaces_skips(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let outcomes = vec![
            target("src/a.rs", Outcome::ran(3))?,
            target("src/b.bin", Outcome::skipped("unmatched extension")?)?,
        ];
        let coverage = Coverage::from_outcomes(outcomes);
        assert_eq!(coverage.ran_count, 1);
        assert_eq!(coverage.skipped_count, 1);
        assert!(coverage.require_nonzero_ran().is_ok());
        assert_eq!(coverage.skips.len(), 1);
        assert_eq!(coverage.skips[0].reason.as_str(), "unmatched extension");
        Ok(())
    }

    #[test]
    fn all_ran_passes_gate_with_no_skips() -> Result<(), Box<dyn std::error::Error>> {
        let outcomes = vec![
            target("src/a.rs", Outcome::ran(3))?,
            target("src/b.rs", Outcome::ran(1))?,
        ];
        let coverage = Coverage::from_outcomes(outcomes);
        assert_eq!(coverage.ran_count, 2);
        assert_eq!(coverage.skipped_count, 0);
        assert!(coverage.require_nonzero_ran().is_ok());
        assert!(coverage.skips.is_empty());
        Ok(())
    }

    #[test]
    fn coverage_wire_form_is_camel_case() -> Result<(), Box<dyn std::error::Error>> {
        let outcomes = vec![target("src/a.rs", Outcome::skipped("empty selection")?)?];
        let coverage = Coverage::from_outcomes(outcomes);
        let wire = serde_json::to_value(&coverage)?;
        assert!(wire.get("ranCount").is_some());
        assert!(wire.get("skippedCount").is_some());
        assert!(wire.get("ran_count").is_none());
        Ok(())
    }

    #[test]
    fn coverage_round_trips_through_json() -> Result<(), Box<dyn std::error::Error>> {
        let outcomes = vec![
            target("src/a.rs", Outcome::ran(2))?,
            target("src/b.rs", Outcome::skipped("missing tool")?)?,
        ];
        let coverage = Coverage::from_outcomes(outcomes);
        let wire = serde_json::to_string(&coverage)?;
        let back: Coverage = serde_json::from_str(&wire)?;
        assert_eq!(back, coverage);
        Ok(())
    }
}
