//! Static-analysis stage (h07): recorded Semgrep/CodeQL/Trivy findings
//! stay SIGNAL-ONLY (non-blocking) unless threat-mapped to an
//! exploitable `ThreatId` — only then does [`StaticThreatGate`] promote
//! them to blocking findings. A finding this gate does not consider
//! exploitable produces nothing from this gate at all: surfacing
//! non-blocking signal is a separate reporting concern, which is what
//! keeps a signal-only fixture clean under the fixture-parity oracle.
//!
//! Raw recorded tool output never enters this module: a malformed or
//! dishonest report is rejected by
//! [`crate::security_pipeline::adapters::static_analysis_report::parse_recorded`],
//! the boundary that mints every branded value used here (including
//! validated `ThreatId` citations — never raw threat text).
//!
//! PROPERTY-TEST: `tests/security_pipeline.rs::
//! recorded_honesty_matrix_property_holds_for_every_stage_shape` drives
//! this stage's parse boundary over a generated shape matrix.

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::{RuleId, ThreatId};
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::security_pipeline::seam::{EngineDetailText, EngineLine, EngineRuleLabel};

/// Threat citations this gate treats as EXPLOITABLE (blocking). A
/// curated, deliberately small allow-list — extending it is a reviewed
/// decision, never a side effect of an engine's own severity label
/// (engines disagree wildly on severity vocabulary; the exploitability
/// judgment lives here, not in the tool).
pub const EXPLOITABLE_THREAT_IDS: &[&str] = &[
    "CWE-89", "CWE-79", "CWE-798", "CWE-502", "T1059", "A03:2021",
];

/// One recorded static-analysis finding, fully branded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticFinding {
    /// The engine rule that fired.
    pub rule: EngineRuleLabel,
    /// Target-relative file the finding points at.
    pub file: RelPath,
    /// 1-based line the engine reported.
    pub line: EngineLine,
    /// Human-readable finding detail.
    pub message: EngineDetailText,
    /// Validated threat citation, when the engine supplied one.
    pub threat: Option<ThreatId>,
}

/// Honest three-way outcome of one recorded static-analysis run
/// (`skipped != passed != failed`; see the adapters module docs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaticOutcome {
    /// The static-analysis tool was not found. Never a pass.
    Skipped {
        // BRAND-INVARIANT: targets covered by the run; always 0 for a
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
        // BRAND-INVARIANT: targets covered by the run, as recorded.
        ran: u32,
        /// The branded findings the run reported.
        findings: Vec<StaticFinding>,
    },
}

/// Gate that promotes ONLY exploitable-threat-mapped static findings to
/// blocking; every other static finding stays signal-only (this gate
/// emits nothing for it).
#[derive(Debug)]
pub struct StaticThreatGate {
    rule_id: RuleId,
}

impl StaticThreatGate {
    /// Build a gate that reports its findings under `rule_id`.
    pub fn new(rule_id: RuleId) -> Self {
        Self { rule_id }
    }

    /// Judge one already-parsed [`StaticOutcome`]: every finding whose
    /// threat citation sits in [`EXPLOITABLE_THREAT_IDS`] yields one
    /// blocking finding; everything else stays signal-only.
    pub fn evaluate(&self, outcome: &StaticOutcome, file: &RelPath) -> Vec<Finding> {
        let StaticOutcome::Ran { findings, .. } = outcome else {
            return Vec::new();
        };
        findings
            .iter()
            .filter_map(|finding| {
                let threat = finding.threat.as_ref()?;
                if !EXPLOITABLE_THREAT_IDS.contains(&threat.as_str()) {
                    return None;
                }
                let title = String::from("static finding threat-mapped to an exploitable weakness");
                Some(Finding {
                    // CLONE-JUSTIFICATION: each finding owns its rule id
                    // and file so the report outlives this borrowed
                    // gate/input.
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Error,
                    title,
                    detail: format!("{} [{threat}]: {}", finding.rule.0, finding.message.0),
                    // CLONE-JUSTIFICATION: same owned-report rationale as
                    // `rule_id` above.
                    file: file.clone(),
                    line: finding.line.0,
                    snippet: None,
                })
            })
            .collect()
    }
}

impl Validator for StaticThreatGate {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    /// Treats `input.source` as one recorded static-analysis report — no
    /// live tool run is required in CI. A report the adapters boundary
    /// rejects (malformed or dishonest) is itself a blocking finding,
    /// never a silent pass.
    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        match crate::security_pipeline::adapters::static_analysis_report::parse_recorded(
            input.source,
        ) {
            Ok(outcome) => self.evaluate(&outcome, input.file),
            Err(rejection) => {
                let title = String::from("static adapter output rejected");
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
