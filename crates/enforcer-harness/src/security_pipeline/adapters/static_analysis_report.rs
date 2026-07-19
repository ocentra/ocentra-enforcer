//! BOUNDARY parser for recorded static-analysis reports (Semgrep/CodeQL/
//! Trivy — all normalized into one recorded JSON shape).
//!
//! BOUNDARY-INVARIANT: [`parse_recorded`] accepts raw recorded JSON and
//! either returns a fully branded
//! [`crate::security_pipeline::static_analysis::StaticOutcome`] or
//! rejects the text as malformed/dishonest with a typed decode failure.
//! Threat citations are validated into the `enforcer_domain` `ThreatId`
//! brand here (MITRE/CWE/OWASP formats only); file paths are validated
//! into `RelPath`. Neither ever travels inward as raw text.
//!
//! boundaryOwnerNote: h07 `security_pipeline` owns this parsing seam.
//!
//! PROPERTY-TEST: `tests/security_pipeline.rs::
//! recorded_honesty_matrix_property_holds_for_every_stage_shape` drives
//! the shared honesty check this parser applies.

use enforcer_core::error::Result;
use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::harness_types::{
    HarnessDiagnosticMessage, HarnessExternalRuleId, HarnessSourceLine,
};
use enforcer_domain::ids::ThreatId;
use enforcer_domain::paths::RelPath;

use crate::security_pipeline::static_analysis::{StaticFinding, StaticOutcome};

/// Raw wire shape of one recorded static-analysis report.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct StaticRecord {
    tool_present: bool,
    outcome: String,
    ran: u32,
    error_message: Option<String>,
    // DEFAULT-JUSTIFICATION: a skipped/errored report legitimately omits
    // the findings array; an absent array means "no findings recorded".
    #[serde(default)]
    findings: Vec<StaticFindingRecord>,
}

/// Raw wire shape of one recorded static-analysis finding.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct StaticFindingRecord {
    rule_id: String,
    file: String,
    line: u32,
    message: String,
    threat_id: Option<String>,
}

/// Parse one recorded static-analysis report into a branded
/// [`StaticOutcome`], rejecting malformed JSON, dishonest shapes,
/// unnamed rules, invalid file paths, and unrecognized threat formats.
///
/// # Errors
/// Returns a typed decode failure naming the violated invariant.
pub fn parse_recorded(raw: &str) -> Result<StaticOutcome> {
    let record: StaticRecord = serde_json::from_str(raw)
        .map_err(|source| DecodeError::new("securityPipeline.static", format!("{source}")))?;

    super::reject_dishonest_shape(
        record.tool_present,
        &record.outcome,
        record.error_message.is_some(),
    )?;

    match record.outcome.as_str() {
        "skipped" => Ok(StaticOutcome::Skipped { ran: record.ran }),
        "errored" => Ok(StaticOutcome::Errored {
            error_message: record
                .error_message
                .unwrap_or_else(|| String::from("the recorded report carried no error message")),
        }),
        "ran" => Ok(StaticOutcome::Ran {
            ran: record.ran,
            findings: record
                .findings
                .into_iter()
                .map(finding_from_record)
                .collect::<Result<Vec<StaticFinding>>>()?,
        }),
        other => Err(DecodeError::new(
            "securityPipeline.static.outcome",
            format!("unrecognized outcome `{other}` — expected skipped/errored/ran"),
        )
        .into()),
    }
}

/// Validate one raw finding record into its branded form.
fn finding_from_record(record: StaticFindingRecord) -> Result<StaticFinding> {
    if record.rule_id.trim().is_empty() {
        return Err(DecodeError::new(
            "securityPipeline.static.ruleId",
            "a finding must name the engine rule that fired — an unnameable finding is malformed",
        )
        .into());
    }
    let threat = match record.threat_id {
        None => None,
        Some(raw_threat) => Some(raw_threat.parse::<ThreatId>()?),
    };
    Ok(StaticFinding {
        rule: HarnessExternalRuleId::try_new(record.rule_id)?,
        file: record.file.parse::<RelPath>()?,
        line: HarnessSourceLine::from_external(u64::from(record.line)),
        message: HarnessDiagnosticMessage::try_new(record.message)?,
        threat,
    })
}
