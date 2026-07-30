//! Native generated-artifact path policy.  This is deliberately path based:
//! generated output is rejected because of where it is committed, not because
//! a fixture happens to contain a marker string.

use std::num::NonZeroU32;
use std::process::Command;

use enforcer_domain::findings::{
    Finding, FindingDetail, FindingLine, FindingSnippet, FindingTitle, Report, ReportOutcome,
    ScanScope, Violation,
};
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::{RelPath, RepoRoot};
use enforcer_domain::severity::Severity;
use enforcer_domain::telemetry_types::SourceLine;

/// Match the frozen MJS generated-output classifier.
pub fn is_generated_artifact_path(path: &str) -> bool {
    let path = path.replace('\\', "/");
    path.starts_with("output/")
        || path.starts_with("test-results/")
        || path.starts_with("playwright-report/")
        || path
            .split('/')
            .any(|part| matches!(part, "dist" | "build" | "coverage" | "generated"))
}

/// Produce the dedicated check report. `tracked` opts into the frozen MJS
/// `git ls-files` inventory behaviour; without it only the supplied scope is
/// evaluated, which keeps an explicit file check deterministic.
pub fn check(
    root: &RepoRoot,
    scope: ScanScope,
    files: &[RelPath],
    tracked: bool,
    allowlist: &[String],
) -> Result<Report, String> {
    let candidates = if tracked {
        tracked_files(root)?
    } else {
        files.to_vec()
    };
    let rule_id: RuleId = "GEN-1.2"
        .parse()
        .map_err(|error: enforcer_domain::boundary::decode_error::DecodeError| error.to_string())?;
    let mut findings = candidates
        .into_iter()
        .filter(|file| {
            is_generated_artifact_path(file.as_str())
                && !allowlist
                    .iter()
                    .any(|pattern| glob_matches(pattern, file.as_str()))
        })
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

fn tracked_files(root: &RepoRoot) -> Result<Vec<RelPath>, String> {
    let output = Command::new("git")
        .current_dir(root.as_str())
        .args(["ls-files", "-z"])
        .output()
        .map_err(|error| format!("cannot execute git ls-files: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "git ls-files failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout)
        .map_err(|error| format!("git ls-files emitted non-UTF-8 paths: {error}"))?
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(|path| RelPath::try_from(path.to_owned()).map_err(|error| error.to_string()))
        .collect()
}

fn finding(rule_id: RuleId, file: RelPath) -> Option<Finding> {
    let line = SourceLine::try_new(NonZeroU32::new(1)?);
    Some(Finding { rule_id, severity: Severity::Error, title: FindingTitle::new("tracked generated artifact path is in source control".to_owned()).ok()?, detail: FindingDetail::new("Remove generated output from source control or configure a narrow generatedArtifactsAllowlist entry.".to_owned()).ok()?, snippet: FindingSnippet::new(file.as_str().to_owned()).ok(), file, line: FindingLine::known(line) })
}

// The frozen config uses minimatch-style allowlist patterns.  This compact
// matcher covers the path glob subset used by that boundary (`*` and `**`).
fn glob_matches(pattern: &str, path: &str) -> bool {
    fn matches(pattern: &[u8], path: &[u8]) -> bool {
        let Some((&pattern_head, pattern_tail)) = pattern.split_first() else {
            return path.is_empty();
        };
        if pattern_head == b'*' {
            let (is_double_star, remaining_pattern) = match pattern_tail.split_first() {
                Some((b'*', tail)) => (true, tail),
                _ => (false, pattern_tail),
            };
            let matches_empty = matches(remaining_pattern, path);
            let Some((&path_head, path_tail)) = path.split_first() else {
                return matches_empty;
            };
            let can_consume = is_double_star || path_head != b'/';
            return matches_empty || (can_consume && matches(pattern, path_tail));
        }
        match path.split_first() {
            Some((&path_head, path_tail)) if pattern_head == path_head => {
                matches(pattern_tail, path_tail)
            }
            _ => false,
        }
    }
    matches(
        pattern.replace('\\', "/").as_bytes(),
        path.replace('\\', "/").as_bytes(),
    )
}

#[cfg(test)]
mod tests {
    use super::{check, glob_matches, is_generated_artifact_path};
    use enforcer_domain::findings::{ReportOutcome, ScanScope};
    use enforcer_domain::paths::{RelPath, RepoRoot};

    #[test]
    fn classifier_matches_frozen_mjs_roots_and_segments() {
        assert!(is_generated_artifact_path("output/run.json"));
        assert!(is_generated_artifact_path("src/generated/client.rs"));
        assert!(!is_generated_artifact_path("src/domain/order.rs"));
    }
    #[test]
    fn allowlist_is_narrow_and_glob_aware() {
        assert!(glob_matches("docs/generated/**", "docs/generated/api.json"));
        assert!(!glob_matches("docs/generated/**", "src/generated/api.json"));
    }
    #[test]
    fn rejects_generated_scope_path_unless_allowlisted() -> Result<(), Box<dyn std::error::Error>> {
        let root: RepoRoot = "C:/fixture".parse()?;
        let file: RelPath = "output/report.json".parse()?;
        assert_eq!(
            check(&root, ScanScope::Files, &[file.clone()], false, &[])?.ok,
            ReportOutcome::Violations
        );
        assert_eq!(
            check(
                &root,
                ScanScope::Files,
                &[file],
                false,
                &["output/**".to_owned()]
            )?
            .ok,
            ReportOutcome::Clean
        );
        Ok(())
    }
}
