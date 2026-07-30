//! Configured aggregate for the frozen `architecture-policy` check.

use enforcer_domain::config_types::{ArchitecturePolicyCheck, EffectiveConfig};
use enforcer_domain::findings::{
    Finding, FindingDetail, FindingLine, FindingSnippet, FindingTitle, Report, ReportOutcome,
    ScanScope, Violation,
};
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::{RelPath, RepoRoot};
use enforcer_domain::severity::Severity;
use enforcer_domain::telemetry_types::SourceLine;

const DEFAULT_CHECKS: &[&str] = &[
    "reexports",
    "validation-bypass",
    "placeholder-implementation",
    "skipped-focused-tests",
    "weak-assertions",
    "rust-string-boundaries",
    "no-zod-source",
    "no-naked-domain-strings",
    "no-test-doubles",
    "cross-platform-script-commands",
    "generated-artifacts",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchitecturePolicyCheckResult {
    pub check: String,
    pub ok: bool,
    pub unavailable: bool,
    pub violations: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchitecturePolicyAggregate {
    pub report: Report,
    pub checks: Vec<ArchitecturePolicyCheckResult>,
}

pub fn canonical_checks(config: &EffectiveConfig) -> Vec<String> {
    let configured: Vec<&str> = if config.architecture_policy_checks.is_empty() {
        DEFAULT_CHECKS.to_vec()
    } else {
        config
            .architecture_policy_checks
            .iter()
            .map(ArchitecturePolicyCheck::as_str)
            .collect()
    };
    let mut out = Vec::new();
    for raw in configured {
        let value = canonical(raw);
        if value != "architecture-policy" && !out.iter().any(|seen| seen == value) {
            out.push(value.to_owned());
        }
    }
    out
}
fn canonical(value: &str) -> &str {
    match value {
        "check-source-shape" => "source-shape",
        "check-required-tests" => "required-tests",
        "check-single-source-contracts" => "single-source-contracts",
        "check-ai-rule-index" => "ai-rule-index",
        "check-dependency-policy" => "dependency-policy",
        "write-sbom" => "sbom",
        "rust-string-boundaries" => "no-naked-domain-strings",
        other => other,
    }
}

pub fn execute(
    root: &RepoRoot,
    scope: ScanScope,
    files: &[RelPath],
    config: &EffectiveConfig,
) -> Result<ArchitecturePolicyAggregate, String> {
    let mut findings = Vec::new();
    let mut checks = Vec::new();
    for check in canonical_checks(config) {
        let report = match check.as_str() {
            "generated-artifacts" => crate::generated_artifacts::check(
                root,
                scope,
                files,
                matches!(
                    config.generated_artifacts_mode,
                    enforcer_domain::config_types::GeneratedArtifactsMode::Tracked
                ),
                &config
                    .generated_artifacts_allowlist
                    .iter()
                    .map(|glob| glob.as_str().to_owned())
                    .collect::<Vec<_>>(),
            )?,
            "source-shape" => crate::source_shape::check(root, scope, files, config)?,
            name if rule_ids(name).is_some() => {
                let validators =
                    crate::engine::build_family_validators().map_err(|error| error.to_string())?;
                let resolved = enforcer_domain::scan_types::ResolvedScope {
                    kind: scope,
                    repo_root: root.clone(),
                    explicit_paths: Vec::new(),
                    diff_range: None,
                };
                let mut report = crate::engine::run(&resolved, files, &validators);
                let ids = rule_ids(name)
                    .ok_or_else(|| format!("missing native rule mapping for {name}"))?;
                report
                    .findings
                    .retain(|finding| ids.contains(&finding.rule_id.as_str()));
                report
                    .violations
                    .retain(|violation| ids.contains(&violation.finding().rule_id.as_str()));
                report.ok = if report.violations.is_empty() {
                    ReportOutcome::Clean
                } else {
                    ReportOutcome::Violations
                };
                report
            }
            _ => unavailable(root, scope, &check),
        };
        let unavailable = check_result_unavailable(&report);
        checks.push(ArchitecturePolicyCheckResult {
            check: check.clone(),
            ok: report.ok == ReportOutcome::Clean,
            unavailable,
            violations: report.violations.len(),
        });
        findings.extend(report.findings);
    }
    let violations = findings
        .iter()
        .cloned()
        .filter_map(|finding| Violation::try_from(finding).ok())
        .collect::<Vec<_>>();
    Ok(ArchitecturePolicyAggregate {
        report: Report {
            ok: if violations.is_empty() {
                ReportOutcome::Clean
            } else {
                ReportOutcome::Violations
            },
            scope,
            violations,
            warnings: Vec::new(),
            waived: Vec::new(),
            findings,
        },
        checks,
    })
}
fn rule_ids(name: &str) -> Option<&'static [&'static str]> {
    Some(match name {
        "reexports" => &["RR-7.2", "RR-7.3", "TS-1.1"],
        "validation-bypass" => &["RR-2.1", "RR-2.2", "TS-2.1", "PY-1.1", "PY-1.2"],
        "placeholder-implementation" => &["RR-4.2", "RR-4.3", "SRC-1.2"],
        "skipped-focused-tests" => &["TS-3.1", "PY-2.1", "TEST-1.3"],
        "weak-assertions" => &["TEST-1.2"],
        "no-zod-source" => &["TS-1.2"],
        "no-naked-domain-strings" => &["RR-6.1", "RR-6.5", "RR-18.16", "TS-1.3", "PY-1.3"],
        "no-test-doubles" => &["TEST-1.1", "TS-8.8"],
        "cross-platform-script-commands" => &["PORT-1.1"],
        _ => return None,
    })
}
fn unavailable(_root: &RepoRoot, scope: ScanScope, check: &str) -> Report {
    let rule_id = match "ARCH-1.10".parse::<RuleId>() {
        Ok(value) => value,
        Err(_) => return empty_failed(scope),
    };
    let Some(number) = std::num::NonZeroU32::new(1) else {
        return empty_failed(scope);
    };
    let line = SourceLine::try_new(number);
    let Ok(title) =
        FindingTitle::new("configured architecture check has no native executor".to_owned())
    else {
        return empty_failed(scope);
    };
    let Ok(detail) = FindingDetail::new(format!(
        "architecture-policy member `{check}` is not implemented natively"
    )) else {
        return empty_failed(scope);
    };
    let Ok(file) = "ocentra-enforcer.config.json".parse() else {
        return empty_failed(scope);
    };
    let finding = Finding {
        rule_id,
        severity: Severity::Error,
        title,
        detail,
        snippet: FindingSnippet::new(check.to_owned()).ok(),
        file,
        line: FindingLine::known(line),
    };
    let violation = match Violation::try_from(finding.clone()) {
        Ok(value) => value,
        Err(_) => return empty_failed(scope),
    };
    Report {
        ok: ReportOutcome::Violations,
        scope,
        violations: vec![violation],
        warnings: Vec::new(),
        waived: Vec::new(),
        findings: vec![finding],
    }
}
fn empty_failed(scope: ScanScope) -> Report {
    Report {
        ok: ReportOutcome::Violations,
        scope,
        violations: Vec::new(),
        warnings: Vec::new(),
        waived: Vec::new(),
        findings: Vec::new(),
    }
}
fn check_result_unavailable(report: &Report) -> bool {
    report
        .findings
        .iter()
        .any(|finding| finding.rule_id.as_str() == "ARCH-1.10")
}

#[cfg(test)]
mod tests {
    use super::{canonical_checks, execute, RelPath, RepoRoot, ReportOutcome, ScanScope};
    use enforcer_config::load_project_config;

    #[test]
    fn aliases_dedupe_and_self_exclusion_are_real() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        std::fs::write(
            temp.path().join("c.json"),
            r#"{"schemaVersion":2,"profileName":"default","architecturePolicyChecks":["rust-string-boundaries","no-naked-domain-strings","check-source-shape","source-shape","architecture-policy","generated-artifacts"]}"#,
        )?;
        let config = load_project_config(&temp.path().join("c.json"))?;
        assert_eq!(
            canonical_checks(&config),
            vec![
                "no-naked-domain-strings",
                "source-shape",
                "generated-artifacts"
            ]
        );
        Ok(())
    }

    #[test]
    fn every_configured_member_contributes_to_one_failed_aggregate(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        std::fs::write(
            temp.path().join("c.json"),
            r#"{"schemaVersion":2,"profileName":"default","architecturePolicyChecks":["generated-artifacts","missing-native-check"]}"#,
        )?;
        let config = load_project_config(&temp.path().join("c.json"))?;
        let root = temp.path().to_string_lossy().parse::<RepoRoot>()?;
        let files = vec!["target/generated.rs".parse::<RelPath>()?];
        let result = execute(&root, ScanScope::Files, &files, &config)?;
        assert_eq!(result.checks.len(), 2);
        assert!(result
            .checks
            .iter()
            .any(|check| check.check == "missing-native-check" && check.unavailable && !check.ok));
        assert!(result
            .report
            .findings
            .iter()
            .any(|finding| finding.rule_id.as_str() == "ARCH-1.10"));
        assert_ne!(result.report.ok, ReportOutcome::Clean);
        Ok(())
    }
}
