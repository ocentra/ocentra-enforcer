//! Native `AI-1.1` agent-rule index validator.

use std::num::NonZeroU32;

use enforcer_domain::findings::{
    Finding, FindingDetail, FindingLine, FindingTitle, Report, ReportOutcome, ScanScope, Violation,
};
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::{RelPath, RepoRoot};
use enforcer_domain::severity::Severity;
use enforcer_domain::telemetry_types::SourceLine;

const DEFAULT_MAX_LINES: usize = 220;

/// Validate that AI rule files are indexed and remain within reviewed bounds.
pub fn check(
    root: &RepoRoot,
    scope: ScanScope,
    max_lines: Option<usize>,
) -> Result<Report, String> {
    let agents = root_path(root, "AGENTS.md");
    let rules_root = root_path(root, ".ocentra-ai/rules");
    if !agents.is_file() || !rules_root.is_dir() {
        return Ok(report(scope, Vec::new()));
    }
    let mut rules = std::fs::read_dir(&rules_root)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && matches!(
                    path.extension().and_then(|v| v.to_str()),
                    Some("md" | "mdc")
                )
        })
        .collect::<Vec<_>>();
    rules.sort();
    let Some(index) = rules
        .iter()
        .find(|path| {
            path.file_name()
                .and_then(|v| v.to_str())
                .is_some_and(|name| {
                    let lower = name.to_ascii_lowercase();
                    lower.contains("rules") || lower.contains("index")
                })
        })
        .or_else(|| rules.first())
    else {
        return Ok(report(scope, Vec::new()));
    };
    let index = index.clone();
    let agents_text = std::fs::read_to_string(&agents).map_err(|error| error.to_string())?;
    let index_text = std::fs::read_to_string(&index).map_err(|error| error.to_string())?;
    let index_rel = relative(root, &index)?;
    let mut findings = Vec::new();
    if !agents_text.contains(&index_rel) && !agents_text.contains(&index_rel.replace('/', "\\")) {
        findings.push(finding(
            "AGENTS.md",
            1,
            format!("AGENTS.md must reference {index_rel}"),
        )?);
    }
    let limit = max_lines.unwrap_or(DEFAULT_MAX_LINES);
    for rule in rules {
        let rel = relative(root, &rule)?;
        let text = std::fs::read_to_string(&rule).map_err(|error| error.to_string())?;
        if rule != index {
            let child = rule
                .strip_prefix(&rules_root)
                .map_err(|error| error.to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            if !index_text.contains(&child) {
                findings.push(finding(
                    &rel,
                    1,
                    format!("{rel} is not linked from {index_rel}"),
                )?);
            }
        }
        let lines = text.lines().count();
        if lines > limit {
            findings.push(finding(
                &rel,
                limit.saturating_add(1),
                format!("{rel} has {lines} lines; split rule files above {limit}"),
            )?);
        }
    }
    Ok(report(scope, findings))
}
fn root_path(root: &RepoRoot, path: &str) -> std::path::PathBuf {
    std::path::Path::new(root.as_str()).join(path)
}
fn relative(root: &RepoRoot, path: &std::path::Path) -> Result<String, String> {
    path.strip_prefix(root.as_str())
        .map_err(|error| error.to_string())
        .map(|path| path.to_string_lossy().replace('\\', "/"))
}
fn finding(path: &str, line: usize, detail: String) -> Result<Finding, String> {
    Ok(Finding {
        rule_id: "AI-1.1"
            .parse::<RuleId>()
            .map_err(|error| error.to_string())?,
        severity: Severity::Error,
        title: FindingTitle::new("agent rule index violation".to_owned())
            .map_err(|error| error.to_string())?,
        detail: FindingDetail::new(detail).map_err(|error| error.to_string())?,
        snippet: None,
        file: path.parse::<RelPath>().map_err(|error| error.to_string())?,
        line: FindingLine::known(SourceLine::try_new(
            NonZeroU32::new(u32::try_from(line).map_err(|_overflow| "line overflow")?)
                .ok_or("line overflow")?,
        )),
    })
}
fn report(scope: ScanScope, findings: Vec<Finding>) -> Report {
    let violations = findings
        .iter()
        .cloned()
        .filter_map(|finding| Violation::try_from(finding).ok())
        .collect::<Vec<_>>();
    Report {
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
    }
}

#[cfg(test)]
mod tests {
    use super::check;
    use enforcer_domain::findings::{ReportOutcome, ScanScope};
    use enforcer_domain::paths::RepoRoot;
    fn write(
        root: &std::path::Path,
        path: &str,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let target = root.join(path);
        std::fs::create_dir_all(target.parent().ok_or("parent")?)?;
        std::fs::write(target, text)?;
        Ok(())
    }
    fn run(
        root: &std::path::Path,
        max: Option<usize>,
    ) -> Result<enforcer_domain::findings::Report, Box<dyn std::error::Error>> {
        let root: RepoRoot = root.to_string_lossy().parse()?;
        Ok(check(&root, ScanScope::Workspace, max)?)
    }
    #[test]
    fn rejects_unlinked_rules_missing_agents_link_and_long_files(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        write(temp.path(), "AGENTS.md", "# guide\n")?;
        write(temp.path(), ".ocentra-ai/rules/rules.mdc", "linked.mdc\n")?;
        write(temp.path(), ".ocentra-ai/rules/linked.mdc", "# linked\n")?;
        write(temp.path(), ".ocentra-ai/rules/missing.mdc", "a\nb\nc\n")?;
        let report = run(temp.path(), Some(2))?;
        assert_eq!(report.ok, ReportOutcome::Violations);
        assert_eq!(report.findings.len(), 3);
        Ok(())
    }
    #[test]
    fn accepts_agents_link_and_every_indexed_rule() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        write(temp.path(), "AGENTS.md", ".ocentra-ai/rules/rules.mdc\n")?;
        write(temp.path(), ".ocentra-ai/rules/rules.mdc", "linked.mdc\n")?;
        write(temp.path(), ".ocentra-ai/rules/linked.mdc", "# linked\n")?;
        assert_eq!(run(temp.path(), None)?.ok, ReportOutcome::Clean);
        Ok(())
    }
}
