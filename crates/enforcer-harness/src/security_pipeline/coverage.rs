//! Coverage stage (h07): line/branch floors (>=90% / >=80%) plus
//! drop-from-previous-run detection, gated as a T1 fail-CI check.
//!
//! Raw recorded tool output never enters this module: a malformed or
//! dishonest report is rejected by
//! [`crate::security_pipeline::adapters::coverage_report::parse_recorded`], the
//! boundary that mints every branded value used here (see its invalid-
//! input rejection contract). This file owns only the branded types and
//! the floor/drop decision.
//!
//! PROPERTY-TEST: `tests/security_pipeline.rs::
//! coverage_floor_gate_property_over_a_percentage_grid` proves the gate
//! fires exactly when line coverage sits below the floor, across a
//! generated percentage grid.

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// Minimum acceptable line coverage.
pub const LINE_FLOOR_PCT: CoveragePct = CoveragePct(90.0);
/// Minimum acceptable branch coverage.
pub const BRANCH_FLOOR_PCT: CoveragePct = CoveragePct(80.0);

/// A coverage percentage.
// BRAND-INVARIANT: always finite and within 0..=100; minted only by
// `super::adapters::coverage_report`, which rejects out-of-range values as
// malformed, and by the two floor constants above.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct CoveragePct(pub(crate) f64);

/// Coverage metrics one recorded tool run reported, fully branded.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoverageMetrics {
    /// Observed line coverage.
    pub line: CoveragePct,
    /// Observed branch coverage.
    pub branch: CoveragePct,
    /// The immediately-preceding run's line coverage, when the recorded
    /// report carried one — enables drop detection independent of the
    /// absolute floor.
    pub previous_line: Option<CoveragePct>,
}

/// Honest three-way outcome of one recorded coverage run
/// (`skipped != passed != failed`; see the adapters module docs).
#[derive(Debug, Clone, PartialEq)]
pub enum CoverageOutcome {
    /// The coverage tool was not found. Never a pass.
    Skipped {
        // BRAND-INVARIANT: items covered by the run; always 0 for a skip
        // (the adapters boundary rejects a skip that claims coverage).
        ran: u32,
    },
    /// The tool was present but failed to produce a usable report.
    Errored {
        // BRAND-INVARIANT: the engine's own failure rendering, carried
        // verbatim for diagnostics; display-only.
        error_message: String,
    },
    /// The tool ran and reported metrics (whether they clear the floor
    /// is [`CoverageFloorGate`]'s judgment, not this outcome's).
    Ran {
        // BRAND-INVARIANT: items covered by the run, as recorded.
        ran: u32,
        /// The branded metrics the run reported.
        metrics: CoverageMetrics,
    },
}

/// T1 gate: fails CI when line/branch coverage sits below its floor, or
/// when line coverage dropped relative to the previous recorded run.
#[derive(Debug)]
pub struct CoverageFloorGate {
    rule_id: RuleId,
}

impl CoverageFloorGate {
    /// Build a gate that reports its findings under `rule_id`.
    pub fn new(rule_id: RuleId) -> Self {
        Self { rule_id }
    }

    /// Judge one already-parsed [`CoverageOutcome`]. Skipped/errored
    /// outcomes produce no floor findings — honesty about tool absence
    /// is the adapters boundary's separate concern.
    pub fn evaluate(&self, outcome: &CoverageOutcome, file: &RelPath) -> Vec<Finding> {
        let CoverageOutcome::Ran { metrics, .. } = outcome else {
            return Vec::new();
        };
        let mut findings = Vec::new();
        if metrics.line < LINE_FLOOR_PCT {
            let title = String::from("line coverage below floor");
            findings.push(Finding {
                // CLONE-JUSTIFICATION: each finding owns its rule id and
                // file so the report outlives this borrowed gate/input.
                rule_id: self.rule_id.clone(),
                severity: Severity::Error,
                title,
                detail: format!(
                    "line coverage {:.1}% is below the {:.0}% floor",
                    metrics.line.0, LINE_FLOOR_PCT.0
                ),
                // CLONE-JUSTIFICATION: same owned-report rationale as
                // `rule_id` above.
                file: file.clone(),
                line: 1,
                snippet: None,
            });
        }
        if metrics.branch < BRANCH_FLOOR_PCT {
            let title = String::from("branch coverage below floor");
            findings.push(Finding {
                // CLONE-JUSTIFICATION: each finding owns its rule id and
                // file so the report outlives this borrowed gate/input.
                rule_id: self.rule_id.clone(),
                severity: Severity::Error,
                title,
                detail: format!(
                    "branch coverage {:.1}% is below the {:.0}% floor",
                    metrics.branch.0, BRANCH_FLOOR_PCT.0
                ),
                // CLONE-JUSTIFICATION: same owned-report rationale as
                // `rule_id` above.
                file: file.clone(),
                line: 1,
                snippet: None,
            });
        }
        if let Some(previous) = metrics.previous_line {
            if metrics.line < previous {
                let title = String::from("line coverage dropped from previous run");
                findings.push(Finding {
                    // CLONE-JUSTIFICATION: each finding owns its rule id
                    // and file so the report outlives this borrowed
                    // gate/input.
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Error,
                    title,
                    detail: format!(
                        "line coverage fell from {:.1}% to {:.1}%",
                        previous.0, metrics.line.0
                    ),
                    // CLONE-JUSTIFICATION: same owned-report rationale as
                    // `rule_id` above.
                    file: file.clone(),
                    line: 1,
                    snippet: None,
                });
            }
        }
        findings
    }
}

impl Validator for CoverageFloorGate {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    /// Treats `input.source` as one recorded coverage report — no live
    /// coverage run is required in CI. A report the adapters boundary
    /// rejects is itself a blocking finding, never a silent pass.
    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        match crate::security_pipeline::adapters::coverage_report::parse_recorded(input.source) {
            Ok(outcome) => self.evaluate(&outcome, input.file),
            Err(rejection) => {
                let title = String::from("coverage adapter output rejected");
                vec![Finding {
                    // CLONE-JUSTIFICATION: the finding owns its rule id and
                    // file so the report outlives this borrowed gate/input.
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Error,
                    title,
                    detail: format!("{rejection}"),
                    // CLONE-JUSTIFICATION: same owned-report rationale as
                    // `rule_id` above.
                    file: input.file.clone(),
                    line: 1,
                    snippet: None,
                }]
            }
        }
    }
}
