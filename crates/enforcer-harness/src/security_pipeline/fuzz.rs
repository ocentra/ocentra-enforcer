//! Property/fuzz stage (h07): a failing property or fuzz run MUST
//! persist the seed that reproduces it — [`FuzzSeedGate`] (T1) flags any
//! recorded failure that carries no seed, because an unreproducible
//! counterexample can be neither triaged nor regression-tested.
//!
//! Raw recorded tool output never enters this module: a malformed or
//! dishonest report is rejected by
//! [`crate::security_pipeline::adapters::fuzz_report::parse_recorded`], the
//! boundary that mints every branded value used here (it also rejects
//! blank seeds and unnamed properties as malformed).
//!
//! PROPERTY-TEST: `tests/security_pipeline.rs::
//! recorded_honesty_matrix_property_holds_for_every_stage_shape` drives
//! this stage's parse boundary over a generated shape matrix.

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use enforcer_domain::harness_types::{
    HarnessDiagnosticMessage, HarnessExternalRuleId, HarnessReproductionSeed,
};

/// One recorded property/fuzz failure, fully branded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuzzFailure {
    /// The failing property's name.
    pub property: HarnessExternalRuleId,
    /// The persisted seed that reproduces this failure; its absence is
    /// exactly what [`FuzzSeedGate`] flags.
    pub seed: Option<HarnessReproductionSeed>,
    /// The concrete counterexample, when the tool captured one.
    pub counterexample: Option<HarnessDiagnosticMessage>,
}

/// Honest three-way outcome of one recorded property/fuzz run
/// (`skipped != passed != failed`; see the adapters module docs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FuzzOutcome {
    /// The property/fuzz tool was not found. Never a pass.
    Skipped {
        // BRAND-INVARIANT: cases covered by the run; always 0 for a skip
        // (the adapters boundary rejects a skip that claims coverage).
        ran: u32,
    },
    /// The tool was present but failed to complete/report.
    Errored {
        // BRAND-INVARIANT: the engine's own failure rendering, carried
        // verbatim for diagnostics; display-only.
        error_message: String,
    },
    /// The tool ran; `failures` may legitimately be empty (a clean run).
    Ran {
        // BRAND-INVARIANT: cases covered by the run, as recorded.
        ran: u32,
        /// The branded failures the run reported.
        failures: Vec<FuzzFailure>,
    },
}

/// T1 gate: a recorded property/fuzz failure with no persisted seed is
/// a blocking finding.
#[derive(Debug)]
pub struct FuzzSeedGate {
    rule_id: RuleId,
}

impl FuzzSeedGate {
    /// Build a gate that reports its findings under `rule_id`.
    pub fn new(rule_id: RuleId) -> Self {
        Self { rule_id }
    }

    /// Judge one already-parsed [`FuzzOutcome`]: every failure that
    /// carries no seed yields one blocking finding.
    pub fn evaluate(&self, outcome: &FuzzOutcome, file: &RelPath) -> Vec<Finding> {
        let FuzzOutcome::Ran { failures, .. } = outcome else {
            return Vec::new();
        };
        failures
            .iter()
            .filter(|failure| failure.seed.is_none())
            .filter_map(|failure| {
                let rendered_counterexample = match &failure.counterexample {
                    Some(text) => format!(" (counterexample: {text})"),
                    None => String::new(),
                };
                domain_finding!(
                    // CLONE-JUSTIFICATION: each finding owns its rule id
                    // and file so the report outlives this borrowed
                    // gate/input.
                    self.rule_id.clone(),
                    Severity::Error,
                    "fuzz/property failure missing persisted seed".to_owned(),
                    format!(
                        "property `{}` failed with no persisted seed — the failure cannot be \
                         reproduced or regression-tested{rendered_counterexample}",
                        failure.property
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

impl Validator for FuzzSeedGate {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    /// Treats `input.source` as one recorded property/fuzz report — no
    /// live fuzzing run is required in CI. A report the adapters
    /// boundary rejects (malformed or dishonest) is itself a blocking
    /// finding, never a silent pass.
    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        match crate::security_pipeline::adapters::fuzz_report::parse_recorded(input.source.as_str())
        {
            Ok(outcome) => self.evaluate(&outcome, input.file),
            Err(rejection) => {
                domain_finding!(
                    // CLONE-JUSTIFICATION: the finding owns its rule id and
                    // file so the report outlives this borrowed gate/input.
                    self.rule_id.clone(),
                    Severity::Error,
                    "fuzz adapter output rejected".to_owned(),
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
