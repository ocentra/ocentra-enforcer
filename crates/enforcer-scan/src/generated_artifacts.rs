//! Native generated-artifact path policy.  This is deliberately path based:
//! generated output is rejected because of where it is committed, not because
//! a fixture happens to contain a marker string.

use std::num::NonZeroU32;
use std::path::Path;
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
    // The frozen check always searches the requested source scope for GEN-1.1
    // markers. Tracked mode adds a separate Git inventory for GEN-1.2 rather
    // than widening the marker scan to every tracked file.
    let mut findings = marker_findings(root, files);
    let candidates = if tracked {
        tracked_files(root)?
    } else {
        files.to_vec()
    };
    let rule_id: RuleId = "GEN-1.2"
        .parse()
        .map_err(|error: enforcer_domain::boundary::decode_error::DecodeError| error.to_string())?;
    findings.extend(
        candidates
            .into_iter()
            .filter(|file| {
                let generated = if tracked {
                    is_generated_artifact_path(file.as_str())
                } else {
                    is_scoped_generated_output_path(file.as_str())
                };
                generated
                    && (!tracked
                        || !allowlist
                            .iter()
                            .any(|pattern| glob_matches(pattern, file.as_str())))
            })
            .filter_map(|file| finding(rule_id.clone(), file)),
    );
    findings.sort_by(|left, right| {
        left.file.as_str().cmp(right.file.as_str()).then_with(|| {
            left.snippet
                .as_ref()
                .map_or("", |value| value.as_str())
                .cmp(right.snippet.as_ref().map_or("", |value| value.as_str()))
        })
    });
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

fn is_scoped_generated_output_path(path: &str) -> bool {
    let path = path.replace('\\', "/");
    path.starts_with("output/")
        || path.starts_with("test-results/")
        || path.starts_with("playwright-report/")
}

fn marker_findings(root: &RepoRoot, files: &[RelPath]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for file in files
        .iter()
        .filter(|file| is_generated_marker_source_file(file.as_str()))
    {
        let Ok(source) = std::fs::read_to_string(Path::new(root.as_str()).join(file.as_str()))
        else {
            continue;
        };
        findings.extend(
            source
                .lines()
                .enumerate()
                .filter_map(|(index, line)| marker_finding(file, index + 1, line)),
        );
    }
    findings
}

fn is_generated_marker_source_file(path: &str) -> bool {
    let path = path.replace('\\', "/");
    let lower = path.to_ascii_lowercase();
    let is_source_extension = [
        ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".mts", ".cts", ".py", ".rs",
    ]
    .iter()
    .any(|extension| lower.ends_with(extension));
    is_source_extension
        && !lower.ends_with(".d.ts")
        && !is_test_path(&lower)
        && [
            "src/",
            "apps/",
            "packages/",
            "crates/",
            "tools/",
            "scripts/",
        ]
        .iter()
        .any(|root| lower.starts_with(root))
}

fn is_test_path(path: &str) -> bool {
    path.contains("/test/")
        || path.contains("/tests/")
        || path.contains("/__tests__/")
        || path.starts_with("test/")
        || path.starts_with("tests/")
        || path.ends_with(".test.js")
        || path.ends_with(".test.jsx")
        || path.ends_with(".test.ts")
        || path.ends_with(".test.tsx")
        || path.ends_with(".test.mjs")
        || path.ends_with(".test.cjs")
        || path.ends_with(".test.mts")
        || path.ends_with(".test.cts")
        || path.ends_with(".spec.js")
        || path.ends_with(".spec.jsx")
        || path.ends_with(".spec.ts")
        || path.ends_with(".spec.tsx")
        || path.ends_with(".spec.mjs")
        || path.ends_with(".spec.cjs")
        || path.ends_with(".spec.mts")
        || path.ends_with(".spec.cts")
        || path.rsplit('/').next().is_some_and(|name| {
            name.starts_with("test_") && name.ends_with(".py") || name.ends_with("_test.py")
        })
}

fn marker_finding(file: &RelPath, line_number: usize, source: &str) -> Option<Finding> {
    let comment = source_comment(file.as_str(), source)?;
    let comment = comment.to_ascii_lowercase();
    if !(comment.contains("@generated")
        || comment.contains("<auto-generated>")
        || comment.contains("generated by"))
    {
        return None;
    }
    let line_number = u32::try_from(line_number).ok().and_then(NonZeroU32::new)?;
    Some(Finding {
        rule_id: "GEN-1.1".parse().ok()?,
        severity: Severity::Error,
        title: FindingTitle::new("Generated artifacts must not be committed as source".to_owned())
            .ok()?,
        detail: FindingDetail::new(
            "Generated artifact marker found in tracked source scope.".to_owned(),
        )
        .ok()?,
        snippet: FindingSnippet::new(source.trim().to_owned()).ok(),
        file: file.clone(),
        line: FindingLine::known(SourceLine::try_new(line_number)),
    })
}

fn source_comment<'a>(path: &str, line: &'a str) -> Option<&'a str> {
    if path.ends_with(".py") {
        return line.find('#').and_then(|index| line.get(index..));
    }
    let slash = line.find("//");
    let block = line.find("/*");
    match (slash, block) {
        (Some(left), Some(right)) => line.get(left.min(right)..),
        (Some(index), None) | (None, Some(index)) => line.get(index..),
        (None, None) => None,
    }
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
// matcher covers the path glob subset used by that boundary (`*`, `**`, and `?`).
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
        if pattern_head == b'?' {
            let Some((&path_head, path_tail)) = path.split_first() else {
                return false;
            };
            return path_head != b'/' && matches(pattern_tail, path_tail);
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

    fn generated_comment(marker: &str) -> String {
        let slashes = ['/', '/'].iter().collect::<String>();
        format!("{slashes} {marker}\n")
    }

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
        assert!(glob_matches("output/run?.json", "output/run1.json"));
        assert!(!glob_matches("output/run?.json", "output/run12.json"));
    }

    #[test]
    fn marker_scanning_matches_frozen_source_only_semantics(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        std::fs::create_dir_all(temp.path().join("src"))?;
        std::fs::create_dir_all(temp.path().join("docs"))?;
        std::fs::create_dir_all(temp.path().join("src/tests"))?;
        std::fs::write(
            temp.path().join("src/generated.rs"),
            generated_comment(&["Gener", "ated by a tool"].concat()),
        )?;
        std::fs::write(temp.path().join("docs/policy.md"), "@generated example\n")?;
        std::fs::write(
            temp.path().join("src/tests/example.rs"),
            generated_comment(&["@", "generated"].concat()),
        )?;
        let root: RepoRoot = temp.path().to_string_lossy().parse()?;
        let files = [
            "src/generated.rs".parse()?,
            "docs/policy.md".parse()?,
            "src/tests/example.rs".parse()?,
        ];
        let report = check(&root, ScanScope::Files, &files, false, &[])?;
        assert_eq!(report.ok, ReportOutcome::Violations);
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].rule_id.as_str(), "GEN-1.1");
        assert_eq!(report.findings[0].file.as_str(), "src/generated.rs");
        Ok(())
    }

    #[test]
    fn scope_mode_matches_frozen_root_paths_without_allowlist(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root: RepoRoot = "C:/fixture".parse()?;
        let files = [
            "src/generated/client.rs".parse()?,
            "output/report.json".parse()?,
        ];
        let report = check(
            &root,
            ScanScope::Files,
            &files,
            false,
            &["output/**".to_owned()],
        )?;
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].rule_id.as_str(), "GEN-1.2");
        assert_eq!(report.findings[0].file.as_str(), "output/report.json");
        Ok(())
    }
    #[test]
    fn tracked_mode_honors_generated_path_allowlist() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        std::fs::create_dir_all(temp.path().join("output"))?;
        std::fs::write(temp.path().join("output/report.json"), "{}")?;
        let init = std::process::Command::new("git")
            .arg("init")
            .current_dir(temp.path())
            .status()?;
        assert!(init.success());
        let add = std::process::Command::new("git")
            .args(["add", "output/report.json"])
            .current_dir(temp.path())
            .status()?;
        assert!(add.success());
        let root: RepoRoot = temp.path().to_string_lossy().parse()?;
        let file: RelPath = "output/report.json".parse()?;
        assert_eq!(
            check(
                &root,
                ScanScope::Files,
                std::slice::from_ref(&file),
                true,
                &[]
            )?
            .ok,
            ReportOutcome::Violations
        );
        assert_eq!(
            check(
                &root,
                ScanScope::Files,
                &[file],
                true,
                &["output/**".to_owned()]
            )?
            .ok,
            ReportOutcome::Clean
        );
        Ok(())
    }

    #[test]
    fn tracked_inventory_detects_a_staged_generated_path() -> Result<(), Box<dyn std::error::Error>>
    {
        let temp = tempfile::tempdir()?;
        std::fs::create_dir_all(temp.path().join("output"))?;
        std::fs::write(temp.path().join("output/report.json"), "{}")?;
        let init = std::process::Command::new("git")
            .arg("init")
            .current_dir(temp.path())
            .status()?;
        assert!(init.success());
        let add = std::process::Command::new("git")
            .args(["add", "output/report.json"])
            .current_dir(temp.path())
            .status()?;
        assert!(add.success());
        let root: RepoRoot = temp.path().to_string_lossy().parse()?;
        let report = check(&root, ScanScope::Workspace, &[], true, &[])?;
        assert_eq!(report.ok, ReportOutcome::Violations);
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.file.as_str() == "output/report.json"));
        Ok(())
    }
}
