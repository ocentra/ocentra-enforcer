//! Recorded-report parsing seam (BOUNDARY) for the h07 security stages.
//!
//! BOUNDARY-INVARIANT: every function under `adapters::*` accepts RAW
//! recorded engine output (JSON text captured from an external tool run,
//! or a fixture authored in that captured shape), rejects malformed or
//! dishonest shapes with a typed decode failure, and returns only
//! validated branded values inward. Raw text and raw primitives never
//! cross out of this module tree except inside those validated values.
//!
//! boundaryOwnerNote: h07 `security_pipeline` owns this parsing seam;
//! the sibling stage-gate modules never touch raw recorded text.
//!
//! The shared honesty check below is the a09-style graceful-skip rule
//! (`skipped != passed != failed`): an absent tool must never be
//! reported as having run, a present tool cannot honestly skip, and a
//! present-but-erroring tool must surface its error, never a silent
//! pass. Every stage parser calls it before constructing an outcome, so
//! the invariant cannot drift per stage.

use enforcer_domain::boundary::decode_error::DecodeError;

pub mod concurrency_report;
pub mod coverage_report;
pub mod crypto_localnet_report;
pub mod fuzz_report;
pub mod observability_report;
pub mod static_analysis_report;

/// Reject the dishonest raw shapes shared by every stage record:
///
/// - `outcome: "skipped"` with `toolPresent: true` — a present tool
///   cannot honestly skip.
/// - a non-skip, non-error outcome with `toolPresent: false` — an absent
///   tool must never be reported as having run.
/// - a non-error outcome carrying an `errorMessage` — an erroring tool
///   must surface the error, never a silent pass.
///
/// Returns the specific invariant a malformed record violated; never
/// panics.
pub(crate) fn reject_dishonest_shape(
    tool_present: bool,
    outcome: &str,
    has_error_message: bool,
) -> Result<(), DecodeError> {
    match outcome {
        "skipped" => {
            if tool_present {
                return Err(DecodeError::new(
                    "securityPipeline.outcome",
                    "`outcome: skipped` but `toolPresent: true` — a present tool cannot honestly skip",
                ));
            }
        }
        "errored" => {}
        _ => {
            if !tool_present {
                return Err(DecodeError::new(
                    "securityPipeline.outcome",
                    "dishonest skip: `toolPresent: false` reported a non-skip outcome — an absent \
                     tool must never be reported as having run",
                ));
            }
            if has_error_message {
                return Err(DecodeError::new(
                    "securityPipeline.outcome",
                    "dishonest pass: an `errorMessage` is present but a non-error outcome was \
                     reported — an erroring tool must surface the error, never a silent pass",
                ));
            }
        }
    }
    Ok(())
}
