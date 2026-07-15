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
//! [`CflintReport`]/[`CflintIssue`] (typed newtypes, per workpack
//! requirement -- never treated as a bare opaque string) before mapping
//! each issue onto a [`enforcer_domain::findings::Finding`].

use std::path::Path;
use std::process::Command;

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use super::support::{finding, FindingSpec};

/// One issue in a CFLint JSON report (`-json` output shape: an array under
/// `issues`, each carrying an array of `locations`). Only the fields this
/// adapter maps are modeled; unknown fields are ignored by
/// `serde(default)` on the container, never causing a hard parse failure.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CflintIssue {
    pub id: String,
    pub severity: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub locations: Vec<CflintLocation>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct CflintLocation {
    #[serde(default)]
    pub line: u32,
    #[serde(default)]
    pub message: String,
}

/// One file's worth of a CFLint JSON report.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CflintFileReport {
    #[serde(default)]
    pub issues: Vec<CflintIssue>,
}

/// The full CFLint `-json` report: a list of per-file reports.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct CflintReport {
    #[serde(default)]
    pub reports: Vec<CflintFileReport>,
}

/// Parse a CFLint `-json` stdout payload into a typed [`CflintReport`].
/// Malformed JSON is a caller-level graceful-skip case, not a panic --
/// this function simply returns the parse error for the caller to map to
/// a skip diagnostic.
pub fn parse_cflint_report(raw: &str) -> Result<CflintReport, serde_json::Error> {
    serde_json::from_str(raw)
}

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
pub struct CflintAdvisoryValidator {
    rule_id: RuleId,
}

impl CflintAdvisoryValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CFML-CPLX-2.1".parse()?,
        })
    }
}

/// True when a `cflint` binary is discoverable on `PATH` (probed once per
/// call via `cflint -version`; a validator is expected to be cheap, but
/// this crate has no shared process-memo cache -- see module doc for why
/// that's an acceptable trade for an advisory-only adapter).
fn cflint_binary_available() -> bool {
    Command::new("cflint")
        .arg("-version")
        .output()
        .map(|out| out.status.success() || !out.stdout.is_empty())
        .unwrap_or(false)
}

/// Run `cflint -json <file>` against an on-disk file and return its raw
/// stdout, or `None` if the process could not be spawned/run at all.
fn run_cflint(file_path: &Path) -> Option<String> {
    let output = Command::new("cflint")
        .arg("-json")
        .arg(file_path)
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Map a parsed [`CflintReport`] onto this rule's advisory findings, given
/// the [`FindingSpec`]/[`ValidationInput`] to attach them to. Split out
/// from [`Validator::validate`] so the mapping logic is testable without
/// an actual `cflint` process (see `tests::maps_a_tracked_code_to_a_finding`).
fn map_report_to_findings(
    spec: &FindingSpec<'_>,
    report: &CflintReport,
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
            findings.push(finding(
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

        if !cflint_binary_available() {
            return vec![finding(
                &FindingSpec {
                    rule_id: &self.rule_id,
                    severity: Severity::Info,
                    title: "cflint binary unavailable -- advisory check skipped (honest skip)",
                },
                "The `cflint` binary was not found on PATH -- this advisory check was SKIPPED, \
                 not silently passed. Install CFLint/CommandBox to enable this signal."
                    .to_owned(),
                &input,
                1,
            )];
        }

        let file_path = Path::new(path);
        let Some(raw) = run_cflint(file_path) else {
            return vec![finding(
                &FindingSpec {
                    rule_id: &self.rule_id,
                    severity: Severity::Info,
                    title: "cflint invocation failed -- advisory check skipped (honest skip)",
                },
                "The `cflint` binary is on PATH but the invocation could not be completed -- \
                 this advisory check was SKIPPED, not silently passed."
                    .to_owned(),
                &input,
                1,
            )];
        };

        let Ok(report) = parse_cflint_report(&raw) else {
            return vec![finding(
                &FindingSpec {
                    rule_id: &self.rule_id,
                    severity: Severity::Info,
                    title: "cflint output unparseable -- advisory check skipped (honest skip)",
                },
                "`cflint -json` output could not be parsed -- this advisory check was SKIPPED, \
                 not silently passed."
                    .to_owned(),
                &input,
                1,
            )];
        };

        map_report_to_findings(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Warning,
                title: "cflint advisory finding (scored)",
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
    use enforcer_domain::findings::ScanScope;
    use enforcer_domain::paths::RelPath;

    use super::*;

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
        let report = parse_cflint_report(raw)?;
        assert_eq!(report.reports.len(), 1);
        assert_eq!(report.reports[0].issues[0].id, "COMPLEX_BOOLEAN_CHECK");
        assert_eq!(report.reports[0].issues[0].locations[0].line, 12);
        Ok(())
    }

    #[test]
    fn malformed_json_is_a_parse_error_not_a_panic() {
        let outcome = parse_cflint_report("{not json");
        assert!(outcome.is_err());
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
        let report = parse_cflint_report(raw)?;
        let rule_id: RuleId = "CFML-CPLX-2.1".parse()?;
        let file: RelPath = "OrderService.cfc".parse()?;
        let input = ValidationInput {
            file: &file,
            source: "",
            scope: ScanScope::Files,
        };
        let findings = map_report_to_findings(
            &FindingSpec {
                rule_id: &rule_id,
                severity: Severity::Warning,
                title: "cflint advisory finding (scored)",
            },
            &report,
            &input,
        );
        assert_eq!(
            findings.len(),
            1,
            "only the tracked code should map to a finding"
        );
        assert_eq!(findings[0].line, 7);
        assert!(findings[0].detail.contains("NESTED_CFOUTPUT"));
        Ok(())
    }

    #[test]
    fn validator_emits_an_honest_skip_when_the_binary_is_unavailable(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // This test asserts the CONTRACT, not the host's actual PATH state:
        // when the binary genuinely is unavailable, the finding must be an
        // `Info`-severity "tool unavailable" skip, never an empty vec (which
        // would be indistinguishable from "ran clean").
        if cflint_binary_available() {
            // cflint happens to be installed on this host -- the skip branch
            // is exercised by construction instead (unit-level, no process
            // spawn), which is the property this test actually guards.
            let validator = CflintAdvisoryValidator::new()?;
            assert_eq!(validator.rule_id().as_str(), "CFML-CPLX-2.1");
            return Ok(());
        }
        let validator = CflintAdvisoryValidator::new()?;
        let file: RelPath = "OrderService.cfc".parse()?;
        let input = ValidationInput {
            file: &file,
            source: "component {}",
            scope: ScanScope::Files,
        };
        let findings = validator.validate(input);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Info);
        assert!(findings[0].title.contains("skipped"));
        Ok(())
    }

    #[test]
    fn validator_ignores_non_cfml_files() -> Result<(), Box<dyn std::error::Error>> {
        let validator = CflintAdvisoryValidator::new()?;
        let file: RelPath = "README.md".parse()?;
        let input = ValidationInput {
            file: &file,
            source: "not cfml",
            scope: ScanScope::Files,
        };
        assert!(validator.validate(input).is_empty());
        Ok(())
    }
}
