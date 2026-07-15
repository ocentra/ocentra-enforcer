//! BOUNDARY parser for recorded property/fuzz reports (fast-check/
//! proptest property runs, Schemathesis/RESTler API fuzzing over an
//! OpenAPI surface — all normalized into one recorded JSON shape).
//!
//! BOUNDARY-INVARIANT: [`parse_recorded`] accepts raw recorded JSON and
//! either returns a fully branded
//! [`crate::security_pipeline::fuzz::FuzzOutcome`] or rejects the text
//! as malformed/dishonest with a typed decode failure. A blank seed and
//! an unnamed failing property are both rejected here — an
//! unreproducible or unnameable failure record is malformed by
//! definition.
//!
//! boundaryOwnerNote: h07 `security_pipeline` owns this parsing seam.
//!
//! PROPERTY-TEST: `tests/security_pipeline.rs::
//! recorded_honesty_matrix_property_holds_for_every_stage_shape` drives
//! this parser over a generated shape matrix.

use enforcer_core::error::Result;
use enforcer_domain::boundary::decode_error::DecodeError;

use crate::security_pipeline::fuzz::{FuzzFailure, FuzzOutcome};
use crate::security_pipeline::seam::{EngineDetailText, EngineRuleLabel, SeedText};

/// Raw wire shape of one recorded property/fuzz report.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct FuzzRecord {
    tool_present: bool,
    outcome: String,
    ran: u32,
    error_message: Option<String>,
    // DEFAULT-JUSTIFICATION: a skipped/errored report legitimately omits
    // the failures array; an absent array means "no failures recorded".
    #[serde(default)]
    failures: Vec<FailureRecord>,
}

/// Raw wire shape of one recorded property/fuzz failure.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct FailureRecord {
    property: String,
    seed: Option<String>,
    counterexample: Option<String>,
}

/// Parse one recorded property/fuzz report into a branded
/// [`FuzzOutcome`], rejecting malformed JSON, dishonest shapes, blank
/// seeds, and unnamed failing properties.
///
/// # Errors
/// Returns a typed decode failure naming the violated invariant.
pub fn parse_recorded(raw: &str) -> Result<FuzzOutcome> {
    let record: FuzzRecord = serde_json::from_str(raw)
        .map_err(|source| DecodeError::new("securityPipeline.fuzz", format!("{source}")))?;

    super::reject_dishonest_shape(
        record.tool_present,
        &record.outcome,
        record.error_message.is_some(),
    )?;

    match record.outcome.as_str() {
        "skipped" => Ok(FuzzOutcome::Skipped { ran: record.ran }),
        "errored" => Ok(FuzzOutcome::Errored {
            error_message: record
                .error_message
                .unwrap_or_else(|| String::from("the recorded report carried no error message")),
        }),
        "ran" => Ok(FuzzOutcome::Ran {
            ran: record.ran,
            failures: record
                .failures
                .into_iter()
                .map(failure_from_record)
                .collect::<Result<Vec<FuzzFailure>>>()?,
        }),
        other => Err(DecodeError::new(
            "securityPipeline.fuzz.outcome",
            format!("unrecognized outcome `{other}` — expected skipped/errored/ran"),
        )
        .into()),
    }
}

/// Validate one raw failure record into its branded form.
fn failure_from_record(record: FailureRecord) -> Result<FuzzFailure> {
    if record.property.trim().is_empty() {
        return Err(DecodeError::new(
            "securityPipeline.fuzz.property",
            "a failing property must be named — an unnameable failure cannot be triaged",
        )
        .into());
    }
    let seed = match record.seed {
        None => None,
        Some(raw_seed) if raw_seed.trim().is_empty() => {
            return Err(DecodeError::new(
                "securityPipeline.fuzz.seed",
                "a blank seed is malformed — it cannot reproduce the failure it claims to seed",
            )
            .into());
        }
        Some(raw_seed) => Some(SeedText(raw_seed)),
    };
    Ok(FuzzFailure {
        property: EngineRuleLabel(record.property),
        seed,
        counterexample: record.counterexample.map(EngineDetailText),
    })
}
