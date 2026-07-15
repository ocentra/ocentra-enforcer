//! BOUNDARY parser for recorded concurrency/load reports (k6/Artillery
//! race and broken-under-load checks — all normalized into one recorded
//! JSON shape).
//!
//! BOUNDARY-INVARIANT: [`parse_recorded`] accepts raw recorded JSON and
//! either returns a fully branded
//! [`crate::security_pipeline::concurrency::ConcurrencyOutcome`] or
//! rejects the text as malformed/dishonest with a typed decode failure.
//! The engine's raw severity word is normalized onto the
//! `enforcer_domain` `Severity` scale HERE (engines disagree on
//! vocabulary), failing CLOSED to `Severity::Error` for unrecognized
//! labels — an unknown severity word must never silently downgrade.
//!
//! boundaryOwnerNote: h07 `security_pipeline` owns this parsing seam.
//!
//! PROPERTY-TEST: `tests/security_pipeline.rs::
//! recorded_honesty_matrix_property_holds_for_every_stage_shape` drives
//! the shared honesty check this parser applies.

use enforcer_core::error::Result;
use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;

use crate::security_pipeline::concurrency::{ConcurrencyFinding, ConcurrencyOutcome};
use crate::security_pipeline::seam::{EngineDetailText, EngineLine, EngineRuleLabel};

/// Raw wire shape of one recorded concurrency/load report.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConcurrencyRecord {
    tool_present: bool,
    outcome: String,
    ran: u32,
    error_message: Option<String>,
    // DEFAULT-JUSTIFICATION: a skipped/errored report legitimately omits
    // the findings array; an absent array means "no findings recorded".
    #[serde(default)]
    findings: Vec<ConcurrencyFindingRecord>,
}

/// Raw wire shape of one recorded concurrency finding.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConcurrencyFindingRecord {
    rule_id: String,
    severity: String,
    file: String,
    line: u32,
    message: String,
}

/// Parse one recorded concurrency/load report into a branded
/// [`ConcurrencyOutcome`], rejecting malformed JSON, dishonest shapes,
/// unnamed checks, and invalid file paths.
///
/// # Errors
/// Returns a typed decode failure naming the violated invariant.
pub fn parse_recorded(raw: &str) -> Result<ConcurrencyOutcome> {
    let record: ConcurrencyRecord = serde_json::from_str(raw)
        .map_err(|source| DecodeError::new("securityPipeline.concurrency", format!("{source}")))?;

    super::reject_dishonest_shape(
        record.tool_present,
        &record.outcome,
        record.error_message.is_some(),
    )?;

    match record.outcome.as_str() {
        "skipped" => Ok(ConcurrencyOutcome::Skipped { ran: record.ran }),
        "errored" => Ok(ConcurrencyOutcome::Errored {
            error_message: record
                .error_message
                .unwrap_or_else(|| String::from("the recorded report carried no error message")),
        }),
        "ran" => Ok(ConcurrencyOutcome::Ran {
            ran: record.ran,
            findings: record
                .findings
                .into_iter()
                .map(finding_from_record)
                .collect::<Result<Vec<ConcurrencyFinding>>>()?,
        }),
        other => Err(DecodeError::new(
            "securityPipeline.concurrency.outcome",
            format!("unrecognized outcome `{other}` — expected skipped/errored/ran"),
        )
        .into()),
    }
}

/// Validate one raw finding record into its branded form, normalizing
/// the engine's severity word onto the shared [`Severity`] scale
/// (fail-closed: unknown words become [`Severity::Error`]).
fn finding_from_record(record: ConcurrencyFindingRecord) -> Result<ConcurrencyFinding> {
    if record.rule_id.trim().is_empty() {
        return Err(DecodeError::new(
            "securityPipeline.concurrency.ruleId",
            "a finding must name the engine check that fired — an unnameable finding is malformed",
        )
        .into());
    }
    let severity = match record.severity.to_ascii_lowercase().as_str() {
        "info" | "informational" | "low" => Severity::Info,
        "warning" | "medium" | "moderate" => Severity::Warning,
        _ => Severity::Error,
    };
    Ok(ConcurrencyFinding {
        rule: EngineRuleLabel(record.rule_id),
        severity,
        file: record.file.parse::<RelPath>()?,
        line: EngineLine(record.line),
        message: EngineDetailText(record.message),
    })
}
