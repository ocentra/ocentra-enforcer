//! Concurrency stage (h07): recorded k6/Artillery race and
//! broken-under-load findings, gated by a T2 severity threshold —
//! [`ConcurrencySeverityGate`] blocks on any finding at or above its
//! threshold on the shared `Severity` scale (the engine's raw severity
//! word was already normalized, fail-closed, at the adapters boundary).
//!
//! Raw recorded tool output never enters this module: a malformed or
//! dishonest report is rejected by
//! [`crate::security_pipeline::adapters::concurrency_report::parse_recorded`],
//! the boundary that mints every branded value used here.
//!
//! PROPERTY-TEST: `tests/security_pipeline.rs::
//! recorded_honesty_matrix_property_holds_for_every_stage_shape` drives
//! this stage's parse boundary over a generated shape matrix.

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::security_pipeline::seam::{EngineDetailText, EngineLine, EngineRuleLabel};

/// One recorded concurrency/load finding, fully branded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConcurrencyFinding {
    /// The engine check that fired.
    pub rule: EngineRuleLabel,
    /// Severity on the shared scale (normalized fail-closed at the
    /// adapters boundary; an unrecognized engine word became `Error`).
    pub severity: Severity,
    /// Target-relative file the finding points at.
    pub file: RelPath,
    /// 1-based line the engine reported.
    pub line: EngineLine,
    /// Human-readable finding detail.
    pub message: EngineDetailText,
}

/// Honest three-way outcome of one recorded concurrency/load run
/// (`skipped != passed != failed`; see the adapters module docs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConcurrencyOutcome {
    /// The load-test tool was not found. Never a pass.
    Skipped {
        // BRAND-INVARIANT: checks covered by the run; always 0 for a
        // skip (the adapters boundary rejects a skip claiming coverage).
        ran: u32,
    },
    /// The tool was present but failed to complete/report.
    Errored {
        // BRAND-INVARIANT: the engine's own failure rendering, carried
        // verbatim for diagnostics; display-only.
        error_message: String,
    },
    /// The tool ran; `findings` may legitimately be empty.
    Ran {
        // BRAND-INVARIANT: checks covered by the run, as recorded.
        ran: u32,
        /// The branded findings the run reported.
        findings: Vec<ConcurrencyFinding>,
    },
}

/// T2 scored gate: blocks on any recorded concurrency finding at or
/// above the configured severity threshold.
#[derive(Debug)]
pub struct ConcurrencySeverityGate {
    rule_id: RuleId,
    threshold: Severity,
}

impl ConcurrencySeverityGate {
    /// Build a gate that reports its findings under `rule_id`, blocking
    /// at `threshold` or worse.
    pub fn new(rule_id: RuleId, threshold: Severity) -> Self {
        Self { rule_id, threshold }
    }

    /// Judge one already-parsed [`ConcurrencyOutcome`]: every finding at
    /// or above the threshold yields one blocking finding.
    pub fn evaluate(&self, outcome: &ConcurrencyOutcome, file: &RelPath) -> Vec<Finding> {
        let ConcurrencyOutcome::Ran { findings, .. } = outcome else {
            return Vec::new();
        };
        findings
            .iter()
            .filter(|finding| finding.severity <= self.threshold)
            .map(|finding| Finding {
                // CLONE-JUSTIFICATION: each finding owns its rule id and
                // file so the report outlives this borrowed gate/input.
                rule_id: self.rule_id.clone(),
                severity: Severity::Error,
                title: format!(
                    "concurrency finding at or above `{:?}` threshold",
                    self.threshold
                ),
                detail: format!(
                    "{} ({:?}): {}",
                    finding.rule.0, finding.severity, finding.message.0
                ),
                // CLONE-JUSTIFICATION: same owned-report rationale as
                // `rule_id` above.
                file: file.clone(),
                line: finding.line.0,
                snippet: None,
            })
            .collect()
    }
}

impl Validator for ConcurrencySeverityGate {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    /// Treats `input.source` as one recorded concurrency/load report —
    /// no live load run is required in CI. A report the adapters
    /// boundary rejects (malformed or dishonest) is itself a blocking
    /// finding, never a silent pass.
    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        match crate::security_pipeline::adapters::concurrency_report::parse_recorded(input.source) {
            Ok(outcome) => self.evaluate(&outcome, input.file),
            Err(rejection) => {
                let title = String::from("concurrency adapter output rejected");
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
