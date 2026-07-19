//! Observability stage (h07 §2.7), part 1 of 2: the branded event
//! vocabulary shared by both observability gates, plus the T2 scored
//! [`MoneyPathLoggingGate`] (a money-critical path emitting no security
//! log / correlation id is scored and flagged). The T1 sampling gate
//! lives in [`crate::security_pipeline::observability_sampling`] as its
//! own module.
//!
//! Money-critical classification is consumed read-only from whatever
//! upstream marked the recorded event money-critical (the h01
//! `enforcer-security` classifier, per the workpack's dependency note) —
//! this module acts on that verdict, it does not reimplement the
//! classifier.
//!
//! Raw recorded tool output never enters this module: a malformed or
//! dishonest report is rejected by
//! [`crate::security_pipeline::adapters::observability_report::parse_recorded`],
//! the boundary that mints every branded value used here.
//!
//! PROPERTY-TEST: `tests/security_pipeline.rs::
//! recorded_honesty_matrix_property_holds_for_every_stage_shape` drives
//! the shared honesty rule guarding this stage's parse boundary.

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::{CorrelationId, RuleId};
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use enforcer_domain::harness_types::HarnessEventLabel;

/// Whether an upstream classifier marked the observed path
/// money-critical.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoneyPathClass {
    /// The event sits on a money-critical path.
    MoneyCritical,
    /// An ordinary, non-money path.
    Ordinary,
}

/// Whether a security log line was emitted for the observed event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityLogPresence {
    /// A security log line was emitted.
    Emitted,
    /// No security log line was emitted.
    Missing,
}

/// What kind of observation the event is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    /// A security-relevant event (auth failure, permission denial, ...).
    SecurityEvent,
    /// An ordinary money-path span/observation.
    MoneyPathSpan,
}

/// Whether trace sampling kept or dropped the observed event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplingDisposition {
    /// The event was recorded.
    Recorded,
    /// Sampling dropped the event before it was recorded.
    DroppedBySampling,
}

/// One observed event, fully branded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservabilityEvent {
    /// The label naming this event.
    pub label: HarnessEventLabel,
    /// Money-critical or ordinary (upstream classifier's verdict).
    pub money_class: MoneyPathClass,
    /// Whether a security log line was emitted.
    pub security_log: SecurityLogPresence,
    /// The correlation id stitched to this event, when one propagated.
    pub correlation: Option<CorrelationId>,
    /// Security event or ordinary span.
    pub kind: EventKind,
    /// Whether sampling kept or dropped the event.
    pub sampling: SamplingDisposition,
}

/// Honest three-way outcome of one recorded observability run
/// (`skipped != passed != failed`; see the adapters module docs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservabilityOutcome {
    /// The tracing backend was not reachable/instrumented. Never a pass.
    Skipped {
        // BRAND-INVARIANT: events covered by the run; always 0 for a
        // skip (the adapters boundary rejects a skip claiming coverage).
        ran: u32,
    },
    /// The backend was present but returned no usable event data.
    Errored {
        // BRAND-INVARIANT: the backend's own failure rendering, carried
        // verbatim for diagnostics; display-only.
        error_message: String,
    },
    /// Events were retrieved (legitimately possibly empty).
    Ran {
        // BRAND-INVARIANT: events covered by the run, as recorded.
        ran: u32,
        /// The branded events the run captured.
        events: Vec<ObservabilityEvent>,
    },
}

/// T2 scored gate: a money-critical path with no security log or no
/// correlation id is scored (score + confidence, carried in the finding
/// detail) and flagged.
#[derive(Debug)]
pub struct MoneyPathLoggingGate {
    rule_id: RuleId,
}

impl MoneyPathLoggingGate {
    /// Build a gate that reports its findings under `rule_id`.
    pub fn new(rule_id: RuleId) -> Self {
        Self { rule_id }
    }

    /// Judge one already-parsed [`ObservabilityOutcome`]: every
    /// money-critical event missing a security log and/or a correlation
    /// id yields one scored finding.
    pub fn evaluate(&self, outcome: &ObservabilityOutcome, file: &RelPath) -> Vec<Finding> {
        let ObservabilityOutcome::Ran { events, .. } = outcome else {
            return Vec::new();
        };
        events
            .iter()
            .filter(|event| event.money_class == MoneyPathClass::MoneyCritical)
            .filter_map(|event| {
                let missing_log = event.security_log == SecurityLogPresence::Missing;
                let missing_correlation = event.correlation.is_none();
                let gap = match (missing_log, missing_correlation) {
                    (false, false) => return None,
                    (true, true) => {
                        "emits no security log and carries no correlation id \
                         (score=1.0, confidence=high)"
                    }
                    (true, false) => "emits no security log (score=0.5, confidence=medium)",
                    (false, true) => "carries no correlation id (score=0.5, confidence=medium)",
                };
                let title = String::from("money-critical path missing security log/correlation id");
                domain_finding!(
                    // CLONE-JUSTIFICATION: each finding owns its rule id
                    // and file so the report outlives this borrowed
                    // gate/input.
                    self.rule_id.clone(),
                    Severity::Error,
                    title,
                    format!("money-critical event `{}` {gap}", event.label),
                    // CLONE-JUSTIFICATION: same owned-report rationale as
                    // `rule_id` above.
                    file.clone(),
                    1,
                )
            })
            .collect()
    }
}

impl Validator for MoneyPathLoggingGate {
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
