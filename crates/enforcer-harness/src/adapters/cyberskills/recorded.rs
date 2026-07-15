//! Parses RECORDED tool-output fixtures (`toolPresent`/`outcome`/`ran`/
//! `findings` JSON — the shape captured from a real engine run, or
//! hand-authored to represent one) into an [`super::seam::AdapterOutcome`].
//!
//! This is the boundary the CI test suite exercises: no live engine binary
//! is ever required to prove the gate logic, only a RECORDED findings JSON
//! plus its expected verdict (per the workpack's acceptance section). The
//! live path (spawning the real subprocess) is intentionally out of scope
//! for this pack per its own charter ("build this pack ONLY as the (d)
//! engine-bound skills are actually needed") — this module is the seam a
//! future live-process adapter would also parse its own stdout through.

use enforcer_core::error::Result;
use enforcer_domain::boundary::decode_error::DecodeError;

use super::seam::{AdapterOutcome, EngineFinding};

/// Raw wire shape a recorded fixture (or a live adapter's captured JSON
/// output) is authored in. Deliberately flatter than [`AdapterOutcome`] —
/// this is what a human/tool WRITES; [`parse_recorded`] is the fail-closed
/// boundary that either accepts it as one honest [`AdapterOutcome`] or
/// rejects it, never silently coercing a dishonest shape into a pass.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawRecorded {
    #[serde(default)]
    tool_present: bool,
    outcome: String,
    #[serde(default)]
    ran: u32,
    #[serde(default)]
    error_message: Option<String>,
    #[serde(default)]
    findings: Vec<EngineFinding>,
}

/// Parse one recorded-fixture JSON document into an [`AdapterOutcome`].
///
/// Fails closed (returns `Err`, never fabricates a value) when the raw
/// shape is DISHONEST — specifically, `toolPresent: false` paired with an
/// `outcome` other than `"skipped"` (a tool that is absent yet claims to
/// have `"pass"`ed or `"ran"` is exactly the silent-pass failure mode this
/// whole seam exists to reject). A present tool may legitimately report
/// `"errored"` or `"ran"`; those two are accepted whether or not
/// `toolPresent` is set (a present-but-erroring engine still surfaces its
/// error either way).
pub fn parse_recorded(raw: &str) -> Result<AdapterOutcome> {
    let parsed: RawRecorded = serde_json::from_str(raw)
        .map_err(|err| DecodeError::new("cyberskillsAdapter", err.to_string()))?;

    match parsed.outcome.as_str() {
        "skipped" => {
            if parsed.tool_present {
                return Err(DecodeError::new(
                    "cyberskillsAdapter.outcome",
                    "`outcome: skipped` but `toolPresent: true` — a present tool cannot honestly skip",
                )
                .into());
            }
            Ok(AdapterOutcome::Skipped { ran: parsed.ran })
        }
        "pass" if !parsed.tool_present => Err(DecodeError::new(
            "cyberskillsAdapter.outcome",
            "dishonest skip: `toolPresent: false` reported `outcome: pass` instead of an honest \
             `skipped` outcome with `ran: 0` — an absent tool must never be reported as a pass",
        )
        .into()),
        "pass" if parsed.error_message.is_some() => Err(DecodeError::new(
            "cyberskillsAdapter.outcome",
            "dishonest pass: an `errorMessage` is present but `outcome: pass` was reported — a \
             present-but-erroring tool must surface the error, never a silent pass",
        )
        .into()),
        "errored" => Ok(AdapterOutcome::Errored {
            error_message: parsed
                .error_message
                .unwrap_or_else(|| "adapter reported an error with no message".to_owned()),
        }),
        "ran" | "pass" => Ok(AdapterOutcome::Ran {
            ran: parsed.ran,
            findings: parsed.findings,
        }),
        other => Err(DecodeError::new(
            "cyberskillsAdapter.outcome",
            format!("unrecognized outcome `{other}` — expected skipped/errored/ran/pass"),
        )
        .into()),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_recorded;
    use crate::adapters::cyberskills::seam::AdapterOutcome;

    #[test]
    fn honest_skip_parses_as_skipped() -> Result<(), Box<dyn std::error::Error>> {
        let outcome = parse_recorded(
            r#"{"adapter":"slither","toolPresent":false,"outcome":"skipped","ran":0,"findings":[]}"#,
        )?;
        assert_eq!(outcome, AdapterOutcome::Skipped { ran: 0 });
        assert!(outcome.is_honest());
        Ok(())
    }

    #[test]
    fn dishonest_absent_tool_reporting_pass_is_rejected() {
        let result = parse_recorded(
            r#"{"adapter":"slither","toolPresent":false,"outcome":"pass","ran":0,"findings":[]}"#,
        );
        assert!(
            result.is_err(),
            "an absent tool reporting `pass` must be rejected, not accepted as a silent skip"
        );
    }

    #[test]
    fn present_tool_with_findings_parses_as_ran() -> Result<(), Box<dyn std::error::Error>> {
        let outcome = parse_recorded(
            r#"{"adapter":"slither","toolPresent":true,"outcome":"ran","ran":1,
                "findings":[{"ruleId":"slither.reentrancy-eth","severity":"High",
                "file":"contracts/Vault.sol","line":42,
                "message":"Reentrancy.","threatId":"CWE-841"}]}"#,
        )?;
        let AdapterOutcome::Ran { ran, findings } = outcome else {
            return Err(format!("expected Ran, got {outcome:?}").into());
        };
        assert_eq!(ran, 1);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].threat_id.as_deref(), Some("CWE-841"));
        Ok(())
    }

    #[test]
    fn present_tool_error_is_surfaced_not_silently_passed() -> Result<(), Box<dyn std::error::Error>>
    {
        let outcome = parse_recorded(
            r#"{"adapter":"slither","toolPresent":true,"outcome":"errored","ran":0,
                "errorMessage":"slither exited 2: compilation failed","findings":[]}"#,
        )?;
        let AdapterOutcome::Errored { error_message } = outcome else {
            return Err(format!("expected Errored, got {outcome:?}").into());
        };
        assert!(error_message.contains("compilation failed"));
        Ok(())
    }

    #[test]
    fn present_tool_reporting_pass_alongside_an_error_message_is_rejected() {
        let result = parse_recorded(
            r#"{"adapter":"slither","toolPresent":true,"outcome":"pass","ran":0,
                "errorMessage":"slither exited 2: compilation failed","findings":[]}"#,
        );
        assert!(
            result.is_err(),
            "a present tool that errored must not be reported as a pass"
        );
    }

    #[test]
    fn skipped_outcome_claiming_tool_present_is_rejected() {
        let result = parse_recorded(r#"{"toolPresent":true,"outcome":"skipped","ran":0}"#);
        assert!(result.is_err());
    }

    #[test]
    fn unrecognized_outcome_label_is_rejected() {
        let result = parse_recorded(r#"{"toolPresent":true,"outcome":"maybe","ran":0}"#);
        assert!(result.is_err());
    }

    #[test]
    fn malformed_json_is_rejected_not_panicking() {
        let result = parse_recorded("{not json");
        assert!(result.is_err());
    }
}
