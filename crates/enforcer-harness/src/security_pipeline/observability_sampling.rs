//! Observability stage (h07 §2.7), part 2 of 2: the T1
//! [`SamplingDropGate`] — a security event dropped by trace sampling is
//! a hard finding, because security events must be recorded with
//! sampling disabled. Split from
//! [`crate::security_pipeline::observability`] (which owns the shared
//! event vocabulary and the scored money-path gate) so each module
//! carries exactly one gate.
//!
//! Raw recorded tool output never enters this module: a malformed or
//! dishonest report is rejected by
//! [`crate::security_pipeline::adapters::observability_report::parse_recorded`].
//!
//! PROPERTY-TEST: `tests/security_pipeline.rs::
//! recorded_honesty_matrix_property_holds_for_every_stage_shape` drives
//! the shared honesty rule guarding this stage's parse boundary.

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::security_pipeline::observability::{
    EventKind, ObservabilityOutcome, SamplingDisposition,
};

/// T1 gate: a security event dropped by trace sampling is a hard
/// failure — security events must never be sampled away.
#[derive(Debug)]
pub struct SamplingDropGate {
    rule_id: RuleId,
}

impl SamplingDropGate {
    /// Build a gate that reports its findings under `rule_id`.
    pub fn new(rule_id: RuleId) -> Self {
        Self { rule_id }
    }

    /// Judge one already-parsed [`ObservabilityOutcome`]: every security
    /// event that sampling dropped yields one blocking finding.
    pub fn evaluate(&self, outcome: &ObservabilityOutcome, file: &RelPath) -> Vec<Finding> {
        let ObservabilityOutcome::Ran { events, .. } = outcome else {
            return Vec::new();
        };
        events
            .iter()
            .filter(|event| {
                event.kind == EventKind::SecurityEvent
                    && event.sampling == SamplingDisposition::DroppedBySampling
            })
            .filter_map(|event| {
                let title = String::from("security event dropped by sampling");
                domain_finding!(
                    // CLONE-JUSTIFICATION: each finding owns its rule id and
                    // file so the report outlives this borrowed gate/input.
                    self.rule_id.clone(),
                    Severity::Error,
                    title,
                    format!(
                        "security event `{}` was dropped by trace sampling — security events \
                         must be recorded with sampling disabled",
                        event.label
                    ),
                    // CLONE-JUSTIFICATION: same owned-report rationale as
                    // `rule_id` above.
                    file.clone(),
                    1,
                )
            })
            .collect()
    }
}

impl Validator for SamplingDropGate {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    /// Treats `input.source` as one recorded observability report — no
    /// live tracing backend is required in CI. A report the adapters
    /// boundary rejects (malformed or dishonest) is itself a blocking
    /// finding, never a silent pass.
    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        match crate::security_pipeline::adapters::observability_report::parse_recorded(
            input.source.as_str(),
        ) {
            Ok(outcome) => self.evaluate(&outcome, input.file),
            Err(rejection) => {
                let title = String::from("observability adapter output rejected");
                domain_finding!(
                    // CLONE-JUSTIFICATION: the finding owns its rule id and
                    // file so the report outlives this borrowed gate/input.
                    self.rule_id.clone(),
                    Severity::Error,
                    title,
                    format!("{rejection}"),
                    // CLONE-JUSTIFICATION: same owned-report rationale as
                    // `rule_id` above.
                    input.file.clone(),
                    1,
                )
                .into_iter()
                .collect()
            }
        }
    }
}
