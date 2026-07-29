//! Renders an `enforcer-domain::findings::Report` into the UI data model
//! the frontend presents, at the Rust boundary (arc-24).
//! BOUNDARY-INVARIANT: domain findings are rendered once into typed UI rows;
//! frontend code cannot re-derive report decisions.
//! boundaryOwnerNote: enforcer-ui owns the shared report payload boundary.
//!
//! This is the one piece of behavior arc-24 fully implements (not a
//! mount point): every Track G feature pack (g02's report view, g03's
//! actions, ...) renders through [`UiReportResponse`] rather than each
//! re-deriving its own view of a `Report`. No business logic lives past
//! this boundary in TS — the frontend only displays the payload shape
//! produced here.
//!
//! camelCase wire casing (locked decision, matches `enforcer-domain`).
//! This type derives `ts_rs::TS` so [`crate::ts_export`] regenerates its
//! committed TypeScript binding from here, never hand-written.
//!
//! ROUNDTRIP-TEST: `cross_lang_roundtrip` serializes and deserializes the
//! complete report response and verifies the committed TypeScript field set.

use enforcer_domain::findings::{Finding, Report, Violation};

/// The UI's rendering of one [`Finding`]/[`Violation`] row: the exact
/// shape the report-view/actions frontend needs (ruleId + severity +
/// title/detail + file:line + an optional excerpt), denormalized so the
/// frontend never has to re-derive display fields from the raw domain
/// type.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct UiFindingRowResponse {
    /// Rule that fired, as its wire string (e.g. `"RR-6.1"`).
    pub rule_id: String,
    /// Severity, lowercase wire form (`"error"`/`"warning"`).
    pub severity: String,
    /// Short human title of the rule.
    pub title: String,
    /// Occurrence-specific detail.
    pub detail: String,
    /// Repo-relative file the finding points at.
    pub file: String,
    /// 1-based line number.
    pub line: u32,
    /// Optional offending source excerpt (already redacted upstream).
    pub snippet: Option<String>,
}

/// The UI's rendering of a whole [`Report`]: the payload the served-HTML
/// fallback and the Tauri report view both consume. `ok`/counts are
/// denormalized alongside the row lists so the frontend never counts
/// rows itself (single source of truth for the summary line).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct UiReportResponse {
    /// True when no blocking violations were found.
    pub ok: bool,
    /// What the run covered, as its wire string (e.g. `"workspace"`).
    pub scope: String,
    /// Blocking violation rows.
    pub violations: Vec<UiFindingRowResponse>,
    /// Non-blocking warning rows.
    pub warnings: Vec<UiFindingRowResponse>,
    /// Rows suppressed by an explicit named waiver.
    pub waived: Vec<UiFindingRowResponse>,
    /// Total finding count across violations + warnings + waived
    /// (denormalized so the frontend never has to sum three arrays for
    /// the summary line).
    pub total_count: u32,
}

/// A boundary-rejected request: the malformed-request fail fixture this
/// workpack's proof row requires. Not itself part of the wire payload —
/// callers (g01's serve handler, g04's run-dispatch) return this as an
/// `Err` rather than emitting a partial/best-effort [`UiReportResponse`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MalformedActionError {
    /// Human-readable reason the request was rejected.
    pub reason: String,
}

fn render_finding(finding: &Finding) -> UiFindingRowResponse {
    UiFindingRowResponse {
        rule_id: finding.rule_id.to_string(),
        severity: serde_json::to_value(finding.severity)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default(),
        title: finding.title.to_string(),
        detail: finding.detail.to_string(),
        file: finding.file.to_string(),
        line: finding
            .line
            .source_line()
            .and_then(|line| line.to_string().parse::<u32>().ok())
            .unwrap_or_default(),
        snippet: finding.snippet.as_ref().map(ToString::to_string),
    }
}

fn render_violation(violation: &Violation) -> UiFindingRowResponse {
    render_finding(violation.finding())
}

/// Render a [`Report`] into its [`UiReportResponse`]. Total mapping, no
/// filtering: an empty/clean report renders the empty-state payload
/// (three empty row lists, `totalCount: 0`, `ok: true`) rather than a
/// special-cased variant, so the frontend has exactly one shape to
/// handle.
#[must_use]
pub fn render_report(report: &Report) -> UiReportResponse {
    let violations: Vec<UiFindingRowResponse> =
        report.violations.iter().map(render_violation).collect();
    let warnings: Vec<UiFindingRowResponse> = report.warnings.iter().map(render_finding).collect();
    let waived: Vec<UiFindingRowResponse> = report.waived.iter().map(render_finding).collect();
    let row_count = violations.len() + warnings.len() + waived.len();
    let total_count = u32::try_from(row_count).unwrap_or(u32::MAX);
    let scope = serde_json::to_value(report.scope)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default();
    UiReportResponse {
        ok: matches!(report.ok, enforcer_domain::findings::ReportOutcome::Clean),
        scope,
        violations,
        warnings,
        waived,
        total_count,
    }
}

impl From<&Report> for UiReportResponse {
    fn from(report: &Report) -> Self {
        render_report(report)
    }
}

/// Validate a raw JSON request body destined for a UI action/run-dispatch
/// endpoint (g03/g04's boundary): rejects a request missing `ruleId` or
/// `files`, matching the requirement checklist's malformed-request fail
/// fixture. Feature packs call this (or a stricter descendant) before any
/// write; arc-24 owns only this shared shape check, not the action
/// semantics themselves.
pub fn validate_action_request(body: &serde_json::Value) -> Result<(), MalformedActionError> {
    let obj = body.as_object().ok_or_else(|| MalformedActionError {
        reason: "request body must be a JSON object".to_owned(),
    })?;
    let has_rule_id = obj.get("ruleId").and_then(|v| v.as_str()).is_some();
    if !has_rule_id {
        return Err(MalformedActionError {
            reason: "request body missing string field `ruleId`".to_owned(),
        });
    }
    let has_files = obj
        .get("files")
        .and_then(|v| v.as_array())
        .is_some_and(|a| !a.is_empty());
    if !has_files {
        return Err(MalformedActionError {
            reason: "request body missing non-empty array field `files`".to_owned(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::MalformedActionError;
    use enforcer_domain::boundary::decode_error::DecodeError;
    use enforcer_domain::findings::{
        Finding, FindingLine, Report, ReportOutcome, ScanScope, Violation,
    };
    use enforcer_domain::ids::RuleId;
    use enforcer_domain::paths::RelPath;
    use enforcer_domain::severity::Severity;
    use enforcer_domain::telemetry_types::SourceLine;

    use super::{render_report, validate_action_request};

    fn sample_finding(severity: Severity) -> Result<Finding, DecodeError> {
        Ok(Finding {
            rule_id: "RR-6.1".parse::<RuleId>()?,
            severity,
            title: "No raw string types".parse()?,
            detail: "Raw string in signature.".parse()?,
            file: "crates/x/src/lib.rs".parse::<RelPath>()?,
            line: FindingLine::known(SourceLine::try_new(
                std::num::NonZeroU32::new(12).unwrap_or(std::num::NonZeroU32::MIN),
            )),
            snippet: None,
        })
    }

    /// PASS fixture: a fixture `Report` with a mix of violations/warnings/
    /// waived findings renders into the expected `UiReportResponse` shape
    /// (row counts, camelCase-derived fields, denormalized `totalCount`).
    #[test]
    fn renders_mixed_report_into_expected_payload() -> Result<(), DecodeError> {
        let violation = Violation::try_from(sample_finding(Severity::Error)?)?;
        let warning = sample_finding(Severity::Warning)?;
        let waived = sample_finding(Severity::Warning)?;
        let report = Report {
            ok: ReportOutcome::Violations,
            scope: ScanScope::Files,
            violations: vec![violation],
            warnings: vec![warning],
            waived: vec![waived],
            findings: vec![],
        };

        let payload = render_report(&report);

        assert!(!payload.ok);
        assert_eq!(payload.scope, "files");
        assert_eq!(payload.violations.len(), 1);
        assert_eq!(payload.warnings.len(), 1);
        assert_eq!(payload.waived.len(), 1);
        assert_eq!(payload.total_count, 3);
        assert_eq!(payload.violations[0].rule_id, "RR-6.1");
        assert_eq!(payload.violations[0].severity, "error");
        assert_eq!(payload.violations[0].file, "crates/x/src/lib.rs");
        Ok(())
    }

    /// PASS fixture: an empty/clean report yields the empty-state payload
    /// — zero rows in every list, `totalCount: 0`, `ok: true` — not a
    /// special-cased variant.
    #[test]
    fn renders_clean_report_into_empty_state_payload() {
        let report = Report {
            ok: ReportOutcome::Clean,
            scope: ScanScope::Workspace,
            violations: vec![],
            warnings: vec![],
            waived: vec![],
            findings: vec![],
        };

        let payload = render_report(&report);

        assert!(payload.ok);
        assert_eq!(payload.scope, "workspace");
        assert!(payload.violations.is_empty());
        assert!(payload.warnings.is_empty());
        assert!(payload.waived.is_empty());
        assert_eq!(payload.total_count, 0);
    }

    /// PASS fixture: a well-formed action request (ruleId + non-empty
    /// files) is accepted.
    #[test]
    fn accepts_well_formed_action_request() {
        let body = serde_json::json!({ "ruleId": "RR-6.1", "files": ["crates/x/src/lib.rs"] });
        assert_eq!(validate_action_request(&body), Ok(()));
    }

    /// FAIL fixture: a request missing `ruleId` is rejected.
    #[test]
    fn rejects_request_missing_rule_id() {
        let body = serde_json::json!({ "files": ["crates/x/src/lib.rs"] });
        assert_eq!(
            validate_action_request(&body),
            Err(MalformedActionError {
                reason: "request body missing string field `ruleId`".to_owned(),
            })
        );
    }

    /// FAIL fixture: a request with an empty `files` array is rejected
    /// (matches the requirement checklist's malformed-request case).
    #[test]
    fn rejects_request_with_empty_files() {
        let body = serde_json::json!({ "ruleId": "RR-6.1", "files": [] });
        assert_eq!(
            validate_action_request(&body),
            Err(MalformedActionError {
                reason: "request body missing non-empty array field `files`".to_owned(),
            })
        );
    }

    /// FAIL fixture: a non-object request body is rejected outright.
    #[test]
    fn rejects_non_object_request_body() {
        let body = serde_json::json!("not-an-object");
        assert_eq!(
            validate_action_request(&body),
            Err(MalformedActionError {
                reason: "request body must be a JSON object".to_owned(),
            })
        );
    }
}
