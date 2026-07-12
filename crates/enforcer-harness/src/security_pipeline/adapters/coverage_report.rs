//! BOUNDARY parser for recorded coverage-tool reports (`cargo llvm-cov`/
//! tarpaulin for Rust targets, c8/nyc + vitest/jest for TS targets — all
//! normalized into one recorded JSON shape before they reach this file).
//!
//! BOUNDARY-INVARIANT: [`parse_recorded`] accepts raw recorded JSON and
//! either returns a fully branded
//! [`crate::security_pipeline::coverage::CoverageOutcome`] or rejects the
//! text as malformed/dishonest with a typed decode failure. Percentages
//! are range-checked here (finite, 0..=100) so the
//! `crate::security_pipeline::coverage::CoveragePct` brand can never hold
//! an out-of-range value.
//!
//! boundaryOwnerNote: h07 `security_pipeline` owns this parsing seam.
//!
//! PROPERTY-TEST: `tests/security_pipeline.rs::
//! coverage_floor_gate_property_over_a_percentage_grid` drives this
//! parser over a generated percentage grid.

use enforcer_core::error::{DecodeError, Result};

use crate::security_pipeline::coverage::{CoverageMetrics, CoverageOutcome, CoveragePct};

/// Raw wire shape of one recorded coverage report. Fields are the
/// captured tool output verbatim; validation happens in
/// [`parse_recorded`], never here.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CoverageRecord {
    tool_present: bool,
    outcome: String,
    ran: u32,
    error_message: Option<String>,
    line_pct: Option<f64>,
    branch_pct: Option<f64>,
    previous_line_pct: Option<f64>,
}

/// Parse one recorded coverage report into a branded
/// [`CoverageOutcome`], rejecting malformed JSON, dishonest shapes, and
/// out-of-range percentages.
///
/// # Errors
/// Returns a typed decode failure naming the violated invariant.
pub fn parse_recorded(raw: &str) -> Result<CoverageOutcome> {
    let record: CoverageRecord = serde_json::from_str(raw)
        .map_err(|source| DecodeError::new("securityPipeline.coverage", format!("{source}")))?;

    super::reject_dishonest_shape(
        record.tool_present,
        &record.outcome,
        record.error_message.is_some(),
    )?;

    match record.outcome.as_str() {
        "skipped" => Ok(CoverageOutcome::Skipped { ran: record.ran }),
        "errored" => Ok(CoverageOutcome::Errored {
            error_message: record
                .error_message
                .unwrap_or_else(|| String::from("the recorded report carried no error message")),
        }),
        "ran" => Ok(CoverageOutcome::Ran {
            ran: record.ran,
            metrics: CoverageMetrics {
                line: coverage_pct("securityPipeline.coverage.linePct", record.line_pct)?,
                branch: coverage_pct("securityPipeline.coverage.branchPct", record.branch_pct)?,
                previous_line: match record.previous_line_pct {
                    None => None,
                    Some(value) => Some(coverage_pct(
                        "securityPipeline.coverage.previousLinePct",
                        Some(value),
                    )?),
                },
            },
        }),
        other => Err(DecodeError::new(
            "securityPipeline.coverage.outcome",
            format!("unrecognized outcome `{other}` — expected skipped/errored/ran"),
        )
        .into()),
    }
}

/// Mint one [`CoveragePct`] from a raw recorded percentage, upholding
/// the brand's range invariant (finite, 0..=100).
fn coverage_pct(field: &'static str, raw: Option<f64>) -> Result<CoveragePct> {
    let value = raw.ok_or_else(|| {
        DecodeError::new(
            field,
            "a `ran` outcome must report this coverage percentage",
        )
    })?;
    if !value.is_finite() || !(0.0..=100.0).contains(&value) {
        return Err(DecodeError::new(
            field,
            format!("invalid percentage `{value}` — must be finite and within 0..=100"),
        )
        .into());
    }
    Ok(CoveragePct(value))
}
