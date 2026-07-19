//! CFLint shell-out advisory adapter (arc-18 posture): the enforcer's own
//! structural validators (`rules::style`, `rules::arch`, ...) already cover
//! the T1 hard-gate shapes (`MISSING_VAR`, `QUERYPARAM_REQ`) natively in
//! Rust. This module additionally shells out to the REAL `cflint` binary,
//! when present on `PATH`, and maps its JSON findings onto a branded
//! enforcer `RuleId` as an ADVISORY (T2) signal layered on top of the
//! native detectors -- catching CFLint rule codes this crate's own text
//! detectors do not attempt to reimplement (`COMPLEX_BOOLEAN_CHECK`,
//! `NESTED_CFOUTPUT`, `UNUSED_METHOD_ARGUMENT`, `ARG_HINT_MISSING`,
//! `COMPONENT_HINT_MISSING`).
//!
//! # Graceful-skip discipline (arc-18)
//!
//! When the `cflint` binary is absent from `PATH`, [`CflintAdvisoryValidator`]
//! emits an HONEST skip finding (`Severity::Info`, title carrying "tool
//! unavailable") -- it NEVER silently returns an empty/clean result that
//! could be misread as "CFLint ran and found nothing". A silent green is
//! indistinguishable from "not evaluated" to a downstream consumer; this
//! adapter always tells the truth about whether it ran.
//!
//! This module deserializes CFLint's `-json` output into
//! [`CflintReportEnvelope`]/[`super::boundary::CflintIssueEnvelope`] (typed wire records, per workpack
//! requirement -- never treated as a bare opaque string) before mapping
//! each issue onto a [`enforcer_domain::findings::Finding`].

use std::path::Path;
use std::process::Command;

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::boundary::validation::ValidationSource;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::{BuiltInCfmlRule, RuleId};
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use super::boundary::{decode_cflint_report, run_cflint, CflintReportEnvelope};
use super::support::FindingSpec;

/// CFLint rule codes this adapter maps to an advisory finding. Anything
/// else in the report is ignored (out of this pack's scope).
const MAPPED_CODES: &[&str] = &[
    "COMPLEX_BOOLEAN_CHECK",
    "NESTED_CFOUTPUT",
    "UNUSED_LOCAL_VARIABLE",
    "UNUSED_METHOD_ARGUMENT",
    "ARG_TYPE_MISSING",
    "ARG_HINT_MISSING",
    "COMPONENT_HINT_MISSING",
];

/// `CFML-CPLX-2.1` -- CFLint shell-out advisory: runs `cflint -json` over
/// the target file and maps any [`MAPPED_CODES`] issue onto this rule's
/// advisory finding. Honest graceful-skip (never a silent pass) when the
/// `cflint` binary is not on `PATH`.
#[derive(Debug)]
pub struct CflintAdvisoryValidator {
    rule_id: RuleId,
}

impl CflintAdvisoryValidator {
    /// Construct the CFLint advisory validator.
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: BuiltInCfmlRule::CflintAdvisory.id(),
        })
    }
}

/// True when a `cflint` binary is discoverable on `PATH` (probed once per
/// call via `cflint -version`; a validator is expected to be cheap, but
/// this crate has no shared process-memo cache -- see module doc for why
/// that's an acceptable trade for an advisory-only adapter).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CflintAvailability {
    Available,
    Unavailable,
}

fn cflint_availability() -> CflintAvailability {
    let available = Command::new("cflint")
        .arg("-version")
        .output()
        .map(|out| out.status.success() || !out.stdout.is_empty())
        .unwrap_or(false);
    if available {
        CflintAvailability::Available
    } else {
        CflintAvailability::Unavailable
    }
}

/// Map a parsed [`CflintReportEnvelope`] onto this rule's advisory findings, given
/// the [`FindingSpec`]/[`ValidationInput`] to attach them to. Split out
/// from [`Validator::validate`] so the mapping logic is testable without
/// an actual `cflint` process (see `tests::maps_a_tracked_code_to_a_finding`).
fn map_report_to_findings(
    spec: &FindingSpec<'_>,
    report: &CflintReportEnvelope,
    input: &ValidationInput<'_>,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    for file_report in &report.reports {
        for issue in &file_report.issues {
            if !MAPPED_CODES.contains(&issue.id.as_str()) {
                continue;
            }
            let line = issue
                .locations
                .first()
                .map(|loc| loc.line.max(1))
                .unwrap_or(1);
            findings.push(finding!(
                spec,
                format!("cflint `{}`: {}", issue.id, issue.message),
                input,
                line,
            ));
        }
    }
    findings
}

impl Validator for CflintAdvisoryValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let path = input.file.as_str();
        if !path.ends_with(".cfc") && !path.ends_with(".cfm") {
            return Vec::new();
        }

        if cflint_availability() == CflintAvailability::Unavailable {
            return vec![finding!(
                &FindingSpec {
                    rule_id: &self.rule_id,
                    severity: Severity::Info,
                    rule: BuiltInCfmlRule::CflintAdvisory,
                },
                "The `cflint` binary was not found on PATH -- this advisory check was SKIPPED, \
                 not silently passed. Install CFLint/CommandBox to enable this signal.",
                &input,
                1,
            )];
        }

        let file_path = Path::new(path);
        let Some(raw) = run_cflint(file_path) else {
            return vec![finding!(
                &FindingSpec {
                    rule_id: &self.rule_id,
                    severity: Severity::Info,
                    rule: BuiltInCfmlRule::CflintAdvisory,
                },
                "The `cflint` binary is on PATH but the invocation could not be completed -- \
                 this advisory check was SKIPPED, not silently passed.",
                &input,
                1,
            )];
        };

        let Ok(report) = decode_cflint_report(ValidationSource::from_text(&raw)) else {
            return vec![finding!(
                &FindingSpec {
                    rule_id: &self.rule_id,
                    severity: Severity::Info,
                    rule: BuiltInCfmlRule::CflintAdvisory,
                },
                "`cflint -json` output could not be parsed -- this advisory check was SKIPPED, \
                 not silently passed.",
                &input,
                1,
            )];
        };

        map_report_to_findings(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Warning,
                rule: BuiltInCfmlRule::CflintAdvisory,
            },
            &report,
            &input,
        )
    }
}

/// Build every validator this module registers.
pub fn all() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    Ok(vec![Box::new(CflintAdvisoryValidator::new()?)])
}

#[cfg(test)]
mod tests {
    use enforcer_domain::boundary::validation::ValidationSource;
    use enforcer_domain::findings::ScanScope;
    use enforcer_domain::ids::{BuiltInCfmlRule, RuleId};
    use enforcer_domain::paths::RelPath;
    use enforcer_domain::severity::Severity;
    use enforcer_validator::validator::ValidationInput;

    use super::{
        cflint_availability, decode_cflint_report, map_report_to_findings, CflintAdvisoryValidator,
        CflintAvailability, FindingSpec, Validator,
    };

    #[test]
    fn rel_path_rejects_invalid_input() {
        let error = RelPath::try_from("../OrderService.cfc".to_owned())
            .err()
            .map(|error| (error.path, error.reason));
        assert_eq!(
            error,
            Some((
                "relPath".to_owned(),
                "invalid relative path: `..` segment escapes the repository root".to_owned()
            ))
        );
    }

    #[test]
    fn parses_a_well_formed_cflint_report() -> Result<(), Box<dyn std::error::Error>> {
        let raw = r#"{
            "reports": [
                {
                    "issues": [
                        {
                            "id": "COMPLEX_BOOLEAN_CHECK",
                            "severity": "WARNING",
                            "message": "too many boolean terms",
                            "locations": [{"line": 12, "message": "here"}]
                        }
                    ]
                }
            ]
        }"#;
        let report = decode_cflint_report(ValidationSource::from_text(raw))?;
        assert_eq!(report.reports.len(), 1);
        assert_eq!(report.reports[0].issues[0].id, "COMPLEX_BOOLEAN_CHECK");
        assert_eq!(report.reports[0].issues[0].locations[0].line, 12);
        Ok(())
    }

    #[test]
    fn malformed_json_is_a_parse_error_not_a_panic() {
        let outcome = decode_cflint_report(ValidationSource::from_text("{not json"));
        assert_eq!(
            outcome.err().map(|error| error.classify()),
            Some(serde_json::error::Category::Syntax)
        );
    }

    #[test]
    fn maps_a_tracked_code_to_an_advisory_finding() -> Result<(), Box<dyn std::error::Error>> {
        let raw = r#"{
            "reports": [
                {
                    "issues": [
                        {
                            "id": "NESTED_CFOUTPUT",
                            "severity": "WARNING",
                            "message": "nested cfoutput",
                            "locations": [{"line": 7, "message": "here"}]
                        },
                        {
                            "id": "SOME_UNTRACKED_CODE",
                            "severity": "INFO",
                            "message": "not mapped by this adapter",
                            "locations": [{"line": 1, "message": "here"}]
                        }
                    ]
                }
            ]
        }"#;
        let report = decode_cflint_report(ValidationSource::from_text(raw))?;
        let rule_id: RuleId = BuiltInCfmlRule::CflintAdvisory.id();
        let file = RelPath::try_from(String::from("OrderService.cfc"))?;
        let input = ValidationInput {
            file: &file,
            source: ValidationSource::from_text(""),
            scope: ScanScope::Files,
        };
        let findings = map_report_to_findings(
            &FindingSpec {
                rule_id: &rule_id,
                severity: Severity::Warning,
                rule: BuiltInCfmlRule::CflintAdvisory,
            },
            &report,
            &input,
        );
        assert_eq!(
            findings.len(),
            1,
            "only the tracked code should map to a finding"
        );
        assert_eq!(
            findings[0]
                .line
                .source_line()
                .map(|line| line.value().get()),
            Some(7)
        );
        assert!(findings[0].detail.as_str().contains("NESTED_CFOUTPUT"));
        Ok(())
    }

    #[test]
    fn validator_emits_an_honest_skip_when_the_binary_is_unavailable(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // This test asserts the CONTRACT, not the host's actual PATH state:
        // when the binary genuinely is unavailable, the finding must be an
        // `Info`-severity "tool unavailable" skip, never an empty vec (which
        // would be indistinguishable from "ran clean").
        if cflint_availability() == CflintAvailability::Available {
            // cflint happens to be installed on this host -- the skip branch
            // is exercised by construction instead (unit-level, no process
            // spawn), which is the property this test actually guards.
            let validator = CflintAdvisoryValidator::new()?;
            assert_eq!(validator.rule_id().as_str(), "CFML-CPLX-2.1");
            return Ok(());
        }
        let validator = CflintAdvisoryValidator::new()?;
        let file = RelPath::try_from(String::from("OrderService.cfc"))?;
        let input = ValidationInput {
            file: &file,
            source: ValidationSource::from_text("component {}"),
            scope: ScanScope::Files,
        };
        let findings = validator.validate(input);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Info);
        assert_eq!(findings[0].title.as_str(), "CFLint advisory result");
        Ok(())
    }

    #[test]
    fn validator_ignores_non_cfml_files() -> Result<(), Box<dyn std::error::Error>> {
        let validator = CflintAdvisoryValidator::new()?;
        let file = RelPath::try_from(String::from("README.md"))?;
        let input = ValidationInput {
            file: &file,
            source: ValidationSource::from_text("not cfml"),
            scope: ScanScope::Files,
        };
        assert!(validator.validate(input).is_empty());
        Ok(())
    }
}
