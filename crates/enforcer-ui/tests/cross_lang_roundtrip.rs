//! Cross-language fixture round-trip test (arc-24's requirement
//! checklist): a fixture `Report` serialized by the Rust backend must
//! deserialize into the exact field set the derived TS type model
//! promises, and back, without loss.
//!
//! There is no TS/JS runtime wired into `cargo test`; this crate has no
//! `node`/`deno`/frontend build step to shell out to (the frontend build
//! type-check is a separate, non-binding proof line per the acceptance
//! criteria). So the "frontend type model" side of the round trip is
//! proven the way a derived-types pipeline actually fails in practice:
//! by parsing the wire JSON as a generic [`serde_json::Value`] and
//! checking it has EXACTLY the field set the committed `UiReportResponse
//! .ts`/`UiFindingRowResponse.ts` declare (camelCase key-for-key) — i.e. a
//! TypeScript consumer typed against the committed bindings could parse
//! this JSON with no missing/extra/renamed field, in either direction.
//! Combined with `tests/ts_drift.rs` (which proves those committed
//! bindings are themselves derived from these exact Rust types, not
//! hand-written), the two tests together close the loop end to end.

use std::collections::BTreeSet;

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::{
    Finding, FindingLine, Report, ReportOutcome, ScanScope, Violation,
};
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_domain::telemetry_types::SourceLine;
use enforcer_ui::payload::{render_report, UiFindingRowResponse, UiReportResponse};

fn fixture_report() -> Result<Report, DecodeError> {
    let finding = Finding {
        rule_id: "RR-6.1".parse::<RuleId>()?,
        severity: Severity::Error,
        title: "No raw string types".parse()?,
        detail: "Raw string in signature.".parse()?,
        file: "crates/x/src/lib.rs".parse::<RelPath>()?,
        line: FindingLine::known(SourceLine::try_new(
            std::num::NonZeroU32::new(12).unwrap_or(std::num::NonZeroU32::MIN),
        )),
        snippet: Some("let x: &str = ...;".parse()?),
    };
    let violation = Violation::try_from(finding.clone())?;
    Ok(Report {
        ok: ReportOutcome::Violations,
        scope: ScanScope::Files,
        violations: vec![violation],
        warnings: vec![],
        waived: vec![],
        findings: vec![finding],
    })
}

/// The exact camelCase field sets the committed bindings declare, kept
/// here as a plain literal (not parsed from the `.ts` files) so this
/// test independently pins the wire contract rather than reading it back
/// out of the same pipeline it is meant to check.
fn expected_ui_report_payload_fields() -> BTreeSet<&'static str> {
    [
        "ok",
        "scope",
        "violations",
        "warnings",
        "waived",
        "totalCount",
    ]
    .into_iter()
    .collect()
}

fn expected_ui_finding_row_fields() -> BTreeSet<&'static str> {
    [
        "ruleId", "severity", "title", "detail", "file", "line", "snippet",
    ]
    .into_iter()
    .collect()
}

fn object_keys(value: &serde_json::Value) -> Option<BTreeSet<String>> {
    Some(value.as_object()?.keys().cloned().collect())
}

/// PASS: the Rust backend's `UiReportResponse` wire JSON has exactly the
/// field set the derived `UiReportResponse.ts`/`UiFindingRowResponse.ts` promise —
/// no field the frontend type model doesn't know about, and no field the
/// frontend type model expects that the backend didn't send.
#[test]
fn ui_report_payload_wire_json_matches_derived_ts_field_set(
) -> Result<(), Box<dyn std::error::Error>> {
    let report = fixture_report()?;
    let payload = render_report(&report);

    let wire = serde_json::to_value(&payload)?;
    let top_level_keys =
        object_keys(&wire).ok_or("UiReportResponse wire JSON must be a JSON object")?;
    let expected_top_level: BTreeSet<String> = expected_ui_report_payload_fields()
        .into_iter()
        .map(str::to_owned)
        .collect();
    assert_eq!(
        top_level_keys, expected_top_level,
        "UiReportResponse wire JSON field set diverges from the derived TS type"
    );

    let violation_row = wire
        .get("violations")
        .and_then(|v| v.get(0))
        .ok_or("wire JSON must contain a violations[0] row")?;
    let row_keys =
        object_keys(violation_row).ok_or("UiFindingRowResponse wire JSON must be a JSON object")?;
    let expected_row: BTreeSet<String> = expected_ui_finding_row_fields()
        .into_iter()
        .map(str::to_owned)
        .collect();
    assert_eq!(
        row_keys, expected_row,
        "UiFindingRowResponse wire JSON field set diverges from the derived TS type"
    );
    Ok(())
}

/// PASS: the fixture `Report`, rendered to `UiReportResponse`, serialized
/// to wire JSON, and deserialized back, round-trips losslessly — every
/// value a TS consumer would read back out matches what the Rust
/// backend rendered, in both directions.
#[test]
fn ui_report_payload_round_trips_without_loss() -> Result<(), Box<dyn std::error::Error>> {
    let report = fixture_report()?;
    let round_trip_payload = render_report(&report);

    let wire = serde_json::to_string(&round_trip_payload)?;
    let back: UiReportResponse = serde_json::from_str(&wire)?;

    assert_eq!(back, round_trip_payload);
    assert_eq!(back.violations.len(), 1);
    let row: &UiFindingRowResponse = back
        .violations
        .first()
        .ok_or("round-tripped payload must contain one violation row")?;
    assert_eq!(row.rule_id, "RR-6.1");
    assert_eq!(row.severity, "error");
    assert_eq!(row.file, "crates/x/src/lib.rs");
    assert_eq!(row.line, 12);
    assert_eq!(row.snippet.as_deref(), Some("let x: &str = ...;"));
    Ok(())
}
