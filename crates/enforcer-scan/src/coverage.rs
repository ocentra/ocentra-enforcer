//! Scan-coverage accounting — aggregates per-target outcome records into ran/skipped counts
//! and enforces the anti-silent-skip
//! invariant: a scan that ran zero checks is a hard failure, never a
//! clean pass.
//!
//! A partial scan (some files skipped for a legitimate reason — unmatched
//! extension, missing tool, unreadable file, empty selection) is fine and
//! stays visible in the report. A scan that ran *nothing* — the hollow
//! self-scan, green because it checked nothing — is not a partial scan;
//! it is a failure this module surfaces explicitly rather than letting an
//! empty findings list masquerade as "all clear".

use enforcer_domain::boundary::decode_error::DecodeError;

use enforcer_domain::paths::RelPath;
use enforcer_domain::scan_types::{Outcome, ScanTargetCount, SkipReason};

/// One target's recorded outcome, keyed by the repo-relative path it
/// applies to. This is the unit `Coverage::from_outcomes` folds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetOutcome {
    /// The file/target this outcome describes.
    pub file: RelPath,
    /// What happened to it.
    pub outcome: Outcome,
}

/// One skip in the scan-domain accounting model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkipRecord {
    /// The file/target that was skipped.
    pub file: RelPath,
    /// Why the scan could not run this target.
    pub reason: SkipReason,
}

/// One skip, surfaced in a report so a partial (or hollow) scan is
/// visible rather than silently absorbed into an empty findings list.
/// See boundary coverage tests for report/DTO roundtrip guarantees.
/// Why. Never empty — see [`SkipReason`].
///
/// Aggregated ran/skipped accounting used by the scan domain.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Coverage {
    /// Number of targets that ran at least the dispatch.
    ran_count: ScanTargetCount,
    /// Number of targets skipped with an explicit reason.
    skipped_count: ScanTargetCount,
    /// Domain skip records retained for coverage enforcement.
    skips: Vec<SkipRecord>,
}

/// Aggregated ran/skipped accounting for one scan run.
///
/// Construct via [`Coverage::from_outcomes`]; the two counters and the
/// skip list are always internally consistent with the outcomes folded
/// in (no field is independently mutable after construction).
/// ROUNDTRIP-TEST: `coverage_roundtrip_through_json` proves this wire shape
/// converts back to the exact scan-domain coverage accounting.
impl Coverage {
    /// Number of targets that ran at least one validator dispatch.
    #[must_use]
    pub const fn ran_count(&self) -> ScanTargetCount {
        self.ran_count
    }

    /// Number of targets skipped with an explicit reason.
    #[must_use]
    pub const fn skipped_count(&self) -> ScanTargetCount {
        self.skipped_count
    }

    /// Domain skip records retained for coverage enforcement.
    #[must_use]
    pub fn skips(&self) -> &[SkipRecord] {
        &self.skips
    }

    /// Rebuild canonical coverage from boundary-validated parts.
    pub(crate) fn from_parts(
        ran_count: ScanTargetCount,
        skipped_count: ScanTargetCount,
        skips: Vec<SkipRecord>,
    ) -> Self {
        Self {
            ran_count,
            skipped_count,
            skips,
        }
    }

    /// Fold a stream of per-target outcomes into aggregated coverage.
    pub fn from_outcomes(outcomes: impl IntoIterator<Item = TargetOutcome>) -> Self {
        let mut coverage = Coverage::default();
        for TargetOutcome { file, outcome } in outcomes {
            match outcome {
                Outcome::Ran { .. } => coverage.ran_count.increment(),
                Outcome::Skipped { reason } => {
                    coverage.skipped_count.increment();
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
    pub const fn total(&self) -> ScanTargetCount {
        self.ran_count.plus(self.skipped_count)
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
        if self.ran_count.is_zero() {
            Err(DecodeError::new(
                "coverage.ran_count",
                format!(
                    "scan ran zero checks (skipped {}) — a scan that checks nothing is not a \
                     clean pass; this is a hard failure (anti-silent-skip)",
                    self.skipped_count.get()
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
    use enforcer_domain::paths::RelPath;
    use enforcer_domain::scan_types::{Outcome, ScanValidatorCount, SkipReason};

    fn target(
        path: RelPath,
        outcome: Outcome,
    ) -> Result<TargetOutcome, Box<dyn std::error::Error>> {
        Ok(TargetOutcome {
            file: path,
            outcome,
        })
    }

    fn ran(count: usize) -> Result<Outcome, Box<dyn std::error::Error>> {
        let count = std::num::NonZeroUsize::new(count)
            .ok_or_else(|| std::io::Error::other("validator count must be positive"))?;
        Ok(Outcome::ran(ScanValidatorCount::try_new(count)))
    }

    fn skipped(reason: &str) -> Result<Outcome, Box<dyn std::error::Error>> {
        Ok(Outcome::skipped(SkipReason::try_new(reason.to_owned())?))
    }

    #[test]
    fn empty_outcomes_yield_zero_and_fail_gate() -> Result<(), Box<dyn std::error::Error>> {
        let coverage = Coverage::from_outcomes(std::iter::empty());
        assert_eq!(coverage.total().get(), 0);
        let error = match coverage.require_nonzero_ran() {
            Ok(()) => {
                return Err(std::io::Error::other(
                    "an empty scan must fail the anti-silent-skip gate",
                )
                .into());
            }
            Err(error) => error,
        };
        assert_eq!(error.path, "coverage.ran_count");
        assert_eq!(
            error.reason,
            "scan ran zero checks (skipped 0) — a scan that checks nothing is not a clean pass; this is a hard failure (anti-silent-skip)"
        );
        Ok(())
    }

    #[test]
    fn all_skipped_fails_the_zero_ran_gate() -> Result<(), Box<dyn std::error::Error>> {
        let outcomes = vec![
            target("src/a.rs".parse()?, skipped("unmatched extension")?)?,
            target("src/b.bin".parse()?, skipped("missing tool")?)?,
        ];
        let coverage = Coverage::from_outcomes(outcomes);
        assert_eq!(coverage.ran_count().get(), 0);
        assert_eq!(coverage.skipped_count().get(), 2);
        assert_eq!(coverage.total().get(), 2);
        let error = match coverage.require_nonzero_ran() {
            Ok(()) => {
                return Err(std::io::Error::other(
                    "all-skipped scan must hard-fail: this is the hollow self-scan",
                )
                .into());
            }
            Err(error) => error,
        };
        assert_eq!(error.path, "coverage.ran_count");
        assert_eq!(
            error.reason,
            "scan ran zero checks (skipped 2) — a scan that checks nothing is not a clean pass; this is a hard failure (anti-silent-skip)"
        );
        Ok(())
    }

    #[test]
    fn mixed_ran_and_skipped_passes_gate_and_surfaces_skips(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let outcomes = vec![
            target("src/a.rs".parse()?, ran(3)?)?,
            target("src/b.bin".parse()?, skipped("unmatched extension")?)?,
        ];
        let coverage = Coverage::from_outcomes(outcomes);
        assert_eq!(coverage.ran_count().get(), 1);
        assert_eq!(coverage.skipped_count().get(), 1);
        coverage.require_nonzero_ran()?;
        assert_eq!(coverage.skips().len(), 1);
        assert_eq!(coverage.skips()[0].reason.as_str(), "unmatched extension");
        Ok(())
    }

    #[test]
    fn all_ran_passes_gate_with_no_skips() -> Result<(), Box<dyn std::error::Error>> {
        let outcomes = vec![
            target("src/a.rs".parse()?, ran(3)?)?,
            target("src/b.rs".parse()?, ran(1)?)?,
        ];
        let coverage = Coverage::from_outcomes(outcomes);
        assert_eq!(coverage.ran_count().get(), 2);
        assert_eq!(coverage.skipped_count().get(), 0);
        coverage.require_nonzero_ran()?;
        assert!(coverage.skips().is_empty());
        Ok(())
    }
}
