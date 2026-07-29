// contractHash: finding_contracts.rs
// sourceOwner: enforcer-domain
use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::{
    Finding, FindingDetail, FindingLine, FindingTitle, Report, ReportOutcome, ScanScope, Violation,
};
use enforcer_domain::severity::Severity;
use enforcer_domain::telemetry_types::SourceLine;

fn sample_finding(severity: Severity) -> Result<Finding, DecodeError> {
    Ok(Finding {
        rule_id: "RR-6.1".parse()?,
        severity,
        title: FindingTitle::new("No raw string types".to_owned())?,
        detail: FindingDetail::new("Raw string in signature.".to_owned())?,
        file: "crates/x/src/lib.rs".parse()?,
        line: FindingLine::known(SourceLine::try_new(
            std::num::NonZeroU32::new(12)
                .ok_or_else(|| DecodeError::new("sourceLine", "expected positive test line"))?,
        )),
        snippet: None,
    })
}

#[test]
fn violation_requires_error_severity() -> Result<(), DecodeError> {
    let blocking = Violation::try_from(sample_finding(Severity::Error)?)?;
    assert_eq!(blocking.finding().severity, Severity::Error);
    let rejection = match Violation::try_from(sample_finding(Severity::Warning)?) {
        Err(error) => error,
        Ok(_) => {
            return Err(DecodeError::new(
                "finding.severity",
                "warning must not become a violation",
            ));
        }
    };
    assert_eq!(rejection.path, "violation.severity");
    Ok(())
}

#[test]
fn finding_wire_form_is_camel_case() -> Result<(), Box<dyn std::error::Error>> {
    let finding = sample_finding(Severity::Error)?;
    let wire = enforcer_domain::boundary::json::to_value(&finding)?;
    assert_eq!(wire["ruleId"], finding.rule_id.to_string());
    assert_eq!(wire.get("rule_id"), None, "snake_case must not leak");
    assert_eq!(wire["file"], "crates/x/src/lib.rs");
    Ok(())
}

#[test]
fn report_round_trips_and_boundary_rejects_bad_violation() -> Result<(), Box<dyn std::error::Error>>
{
    let finding = sample_finding(Severity::Error)?;
    let violation = Violation::try_from(finding.clone())?;
    let report = Report {
        ok: ReportOutcome::Violations,
        scope: ScanScope::Files,
        violations: vec![violation],
        warnings: vec![],
        waived: vec![],
        findings: vec![finding],
    };
    let wire = enforcer_domain::boundary::json::to_string(&report)?;
    let back: Report = enforcer_domain::boundary::json::from_str(&wire)?;
    assert_eq!(back, report);

    // A violation whose severity is not `error` must fail to decode.
    let smuggled = wire.replace("\"severity\":\"error\"", "\"severity\":\"warning\"");
    let rejection = enforcer_domain::boundary::json::from_str::<Report>(&smuggled)
        .err()
        .ok_or("non-error violation severity must be rejected")?;
    assert_eq!(rejection.classify(), serde_json::error::Category::Data);
    Ok(())
}

#[test]
fn scan_scope_wire_form_is_lowercase() -> Result<(), serde_json::Error> {
    assert_eq!(
        enforcer_domain::boundary::json::to_string(&ScanScope::Workspace)?,
        "\"workspace\""
    );
    let parsed: ScanScope = enforcer_domain::boundary::json::from_str("\"diff\"")?;
    assert_eq!(parsed, ScanScope::Diff);
    let rejection = enforcer_domain::boundary::json::from_str::<ScanScope>("\"repo\"")
        .err()
        .ok_or_else(|| serde_json::Error::io(std::io::Error::other("repo scope accepted")))?;
    assert_eq!(rejection.classify(), serde_json::error::Category::Data);
    Ok(())
}
