//! Finding / Violation / Report / ScanScope DTOs — the record shapes the
//! validators emit and the MCP/UI surfaces render. camelCase wire casing
//! (locked decision); `ts_rs::TS` derives feed the arc-24 Rust->TS
//! pipeline.

use enforcer_core::error::DecodeError;

use crate::ids::RuleId;
use crate::paths::RelPath;
use crate::severity::Severity;

/// What a validation run covered.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, ts_rs::TS,
)]
#[serde(rename_all = "lowercase")]
pub enum ScanScope {
    /// Whole workspace.
    Workspace,
    /// An explicit file list.
    Files,
    /// One Cargo crate.
    Crate,
    /// A git diff range.
    Diff,
}

/// One finding produced by a rule against a file location.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    /// Rule that fired.
    pub rule_id: RuleId,
    /// Severity of this occurrence.
    pub severity: Severity,
    /// Short human title of the rule.
    pub title: String,
    /// Occurrence-specific detail.
    pub detail: String,
    /// Repo-relative file the finding points at.
    pub file: RelPath,
    /// 1-based line number.
    pub line: u32,
    /// Optional offending source excerpt (already redacted upstream).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
}

/// A BLOCKING finding. Invariant: severity is [`Severity::Error`], enforced
/// at construction — a non-error finding cannot become a `Violation`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(try_from = "Finding", into = "Finding")]
pub struct Violation(Finding);

impl Violation {
    /// View the underlying finding.
    pub fn finding(&self) -> &Finding {
        &self.0
    }
}

impl TryFrom<Finding> for Violation {
    type Error = DecodeError;

    fn try_from(finding: Finding) -> Result<Self, DecodeError> {
        if finding.severity == Severity::Error {
            Ok(Self(finding))
        } else {
            Err(DecodeError::new(
                "violation.severity",
                "a violation must carry severity `error`",
            ))
        }
    }
}

impl From<Violation> for Finding {
    fn from(violation: Violation) -> Finding {
        violation.0
    }
}

/// The report a check/scan run returns to callers (CLI, MCP, CI).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    /// True when no blocking violations were found.
    pub ok: bool,
    /// What the run covered.
    pub scope: ScanScope,
    /// Blocking violations.
    pub violations: Vec<Violation>,
    /// Non-blocking warnings.
    pub warnings: Vec<Finding>,
    /// Findings suppressed by an explicit waiver.
    pub waived: Vec<Finding>,
    /// Every finding (violations + warnings + waived, denormalized for
    /// consumers that want one list).
    pub findings: Vec<Finding>,
}

#[cfg(test)]
mod tests {
    use super::{Finding, Report, ScanScope, Severity, Violation};
    use enforcer_core::error::DecodeError;

    fn sample_finding(severity: Severity) -> Result<Finding, DecodeError> {
        Ok(Finding {
            rule_id: "RR-6.1".parse()?,
            severity,
            title: "No raw string types".to_owned(),
            detail: "Raw string in signature.".to_owned(),
            file: "crates/x/src/lib.rs".parse()?,
            line: 12,
            snippet: None,
        })
    }

    #[test]
    fn violation_requires_error_severity() -> Result<(), DecodeError> {
        let blocking = Violation::try_from(sample_finding(Severity::Error)?);
        assert!(blocking.is_ok());
        let non_blocking = Violation::try_from(sample_finding(Severity::Warning)?);
        assert!(non_blocking.is_err());
        Ok(())
    }

    #[test]
    fn finding_wire_form_is_camel_case() -> Result<(), DecodeError> {
        let finding = sample_finding(Severity::Error)?;
        let wire = serde_json::to_value(&finding)
            .map_err(|e| DecodeError::new("finding", e.to_string()))?;
        assert!(wire.get("ruleId").is_some(), "camelCase ruleId expected");
        assert!(wire.get("rule_id").is_none(), "snake_case must not leak");
        assert_eq!(wire["file"], "crates/x/src/lib.rs");
        Ok(())
    }

    #[test]
    fn report_round_trips_and_boundary_rejects_bad_violation() -> Result<(), DecodeError> {
        let finding = sample_finding(Severity::Error)?;
        let violation = Violation::try_from(finding.clone())?;
        let report = Report {
            ok: false,
            scope: ScanScope::Files,
            violations: vec![violation],
            warnings: vec![],
            waived: vec![],
            findings: vec![finding],
        };
        let wire = serde_json::to_string(&report)
            .map_err(|e| DecodeError::new("report", e.to_string()))?;
        let back: Report =
            serde_json::from_str(&wire).map_err(|e| DecodeError::new("report", e.to_string()))?;
        assert_eq!(back, report);

        // A violation whose severity is not `error` must fail to decode.
        let smuggled = wire.replace("\"severity\":\"error\"", "\"severity\":\"warning\"");
        assert!(serde_json::from_str::<Report>(&smuggled).is_err());
        Ok(())
    }

    #[test]
    fn scan_scope_wire_form_is_lowercase() -> Result<(), serde_json::Error> {
        assert_eq!(
            serde_json::to_string(&ScanScope::Workspace)?,
            "\"workspace\""
        );
        let parsed: ScanScope = serde_json::from_str("\"diff\"")?;
        assert_eq!(parsed, ScanScope::Diff);
        assert!(serde_json::from_str::<ScanScope>("\"repo\"").is_err());
        Ok(())
    }
}
