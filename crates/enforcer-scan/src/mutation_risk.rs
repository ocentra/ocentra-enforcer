//! Native mutation-risk policy (frozen MJS `ENF-2.1` parity).
//!
//! This is intentionally a path policy, not a source scan. A change to a
//! policy-critical path requires the proof set recorded in the finding.

use std::num::NonZeroU32;

use enforcer_domain::findings::{
    Finding, FindingDetail, FindingLine, FindingSnippet, FindingTitle, Report, ReportOutcome,
    ScanScope, Violation,
};
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_domain::telemetry_types::SourceLine;

pub const REQUIRED_PROOFS: &[&str] = &[
    "ocentra-enforcer scan --workspace",
    "ocentra-enforcer check rule-coverage --root <repo>",
    "ocentra-enforcer check policy-integrity --root <repo>",
    "ocentra-enforcer check ci-integrity --root <repo>",
    "ocentra-enforcer check repo-governance --root <repo>",
    "npm test",
    "npm run test:mcp",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutationRiskPolicy {
    pub critical_patterns: Vec<String>,
}

impl Default for MutationRiskPolicy {
    fn default() -> Self {
        Self {
            critical_patterns: [
                "rules/**",
                "schemas/**",
                "profiles/**",
                "scripts/**",
                "src/policy*",
                "src/checks*",
                "src/generic-scanners*",
                "src/source-policy-scanners*",
                "mcp/**",
                ".github/workflows/**",
                "package.json",
                "package-lock.json",
                "Cargo.toml",
                "Cargo.lock",
                "deny.toml",
                "rust-toolchain.toml",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        }
    }
}

pub fn check(
    scope: ScanScope,
    files: &[RelPath],
    policy: &MutationRiskPolicy,
) -> Result<Report, String> {
    let rule_id: RuleId = "ENF-2.1"
        .parse()
        .map_err(|error: enforcer_domain::boundary::decode_error::DecodeError| error.to_string())?;
    let mut findings = files
        .iter()
        .filter(|file| {
            policy
                .critical_patterns
                .iter()
                .any(|glob| glob_matches(glob, file.as_str()))
        })
        .cloned()
        .filter_map(|file| finding(rule_id.clone(), file))
        .collect::<Vec<_>>();
    findings.sort_by(|left, right| left.file.as_str().cmp(right.file.as_str()));
    let violations = findings
        .iter()
        .cloned()
        .filter_map(|finding| Violation::try_from(finding).ok())
        .collect::<Vec<_>>();
    Ok(Report {
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
    })
}

fn finding(rule_id: RuleId, file: RelPath) -> Option<Finding> {
    Some(Finding {
        rule_id,
        severity: Severity::Error,
        title: FindingTitle::new(format!("policy-critical file changed: {}", file.as_str()))
            .ok()?,
        detail: FindingDetail::new(format!(
            "Required proof set: {}",
            REQUIRED_PROOFS.join("; ")
        ))
        .ok()?,
        snippet: FindingSnippet::new(file.as_str().to_owned()).ok(),
        file,
        line: FindingLine::known(SourceLine::try_new(NonZeroU32::new(1)?)),
    })
}

fn glob_matches(pattern: &str, path: &str) -> bool {
    fn matches(pattern: &[u8], path: &[u8]) -> bool {
        let Some((&head, tail)) = pattern.split_first() else {
            return path.is_empty();
        };
        if head == b'*' {
            let (double, rest) = match tail.split_first() {
                Some((b'*', rest)) => (true, rest),
                _ => (false, tail),
            };
            return matches(rest, path)
                || path.split_first().is_some_and(|(&next, rest_path)| {
                    (double || next != b'/') && matches(pattern, rest_path)
                });
        }
        if head == b'?' {
            return path
                .split_first()
                .is_some_and(|(&next, rest)| next != b'/' && matches(tail, rest));
        }
        path.split_first()
            .is_some_and(|(&next, rest)| head == next && matches(tail, rest))
    }
    matches(
        pattern.replace('\\', "/").as_bytes(),
        path.replace('\\', "/").as_bytes(),
    )
}

#[cfg(test)]
mod tests {
    use super::{check, MutationRiskPolicy};
    use enforcer_domain::findings::ScanScope;
    use enforcer_domain::paths::RelPath;

    #[test]
    fn critical_paths_produce_enf_2_1_with_proof_evidence() -> Result<(), String> {
        let files = [
            "Cargo.lock"
                .parse::<RelPath>()
                .map_err(|error| error.to_string())?,
            "crates/a/src/lib.rs"
                .parse::<RelPath>()
                .map_err(|error| error.to_string())?,
            ".github/workflows/ci.yml"
                .parse::<RelPath>()
                .map_err(|error| error.to_string())?,
        ];
        let report = check(ScanScope::Diff, &files, &MutationRiskPolicy::default())?;
        assert_eq!(report.violations.len(), 2);
        assert!(report
            .findings
            .iter()
            .all(|finding| finding.rule_id.as_str() == "ENF-2.1"));
        assert!(report.findings[0]
            .detail
            .as_str()
            .contains("npm run test:mcp"));
        Ok(())
    }
    #[test]
    fn custom_policy_is_the_only_path_matcher() -> Result<(), String> {
        let files = [
            "engine/policy.rs"
                .parse::<RelPath>()
                .map_err(|error| error.to_string())?,
            "Cargo.toml"
                .parse::<RelPath>()
                .map_err(|error| error.to_string())?,
        ];
        let policy = MutationRiskPolicy {
            critical_patterns: vec!["engine/**".to_owned()],
        };
        assert_eq!(
            check(ScanScope::Files, &files, &policy)?.violations.len(),
            1
        );
        Ok(())
    }
}
