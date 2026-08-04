//! Native frozen-MJS parity for the `docs-completeness` standalone check.

use enforcer_domain::findings::{
    Finding, FindingDetail, FindingLine, FindingSnippet, FindingTitle, Report, ReportOutcome,
    ScanScope, Violation,
};
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::{RelPath, RepoRoot};
use enforcer_domain::severity::Severity;
use enforcer_domain::telemetry_types::SourceLine;
use std::num::NonZeroU32;
use std::path::Path;

#[derive(serde::Deserialize)]
struct Catalog {
    rules: Vec<Rule>,
}
#[derive(serde::Deserialize)]
struct Rule {
    id: String,
    language: String,
    family: String,
    #[serde(rename = "lockLevel")]
    lock_level: String,
    validator: String,
    doc: String,
    snippet: String,
}

/// Validate that every catalog rule has complete indexed documentation.
pub fn check(root: &RepoRoot, scope: ScanScope) -> Result<Report, String> {
    let catalog_path = Path::new(root.as_str()).join("rules/rules.json");
    let catalog: Catalog =
        serde_json::from_str(&std::fs::read_to_string(&catalog_path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let mut findings = Vec::new();
    for file in markdown_files(&Path::new(root.as_str()).join("rules"))? {
        if file.file_name().and_then(|v| v.to_str()) == Some("INDEX.md") {
            continue;
        }
        let text = std::fs::read_to_string(&file).map_err(|e| e.to_string())?;
        let rel = relative(root, &file)?;
        let docs = catalog
            .rules
            .iter()
            .filter(|rule| rule.doc.split('#').next() == Some(rel.as_str()))
            .collect::<Vec<_>>();
        let missing = [
            "Covered Rules",
            "Fails",
            "Passes",
            "Fix Recipe",
            "Validator",
        ]
        .into_iter()
        .filter(|heading| !has_heading(&text, heading))
        .collect::<Vec<_>>();
        if !missing.is_empty() {
            findings.push(finding(
                "DOCENF-1.1",
                &rel,
                1,
                format!("{rel} is missing rule doc sections: {}", missing.join(", ")),
                None,
            )?);
        }
        if docs.iter().any(|r| {
            matches!(r.language.as_str(), "rust" | "typescript" | "python")
                && matches!(r.family.as_str(), "source" | "domain" | "imports-modules")
        }) && !has_fail_pass_code(&text)
        {
            findings.push(finding(
                "DOCENF-1.2",
                &rel,
                1,
                format!(
                    "{rel} covers source rules but lacks both fail and pass fenced code examples"
                ),
                None,
            )?);
        }
        if docs.iter().any(|r| r.lock_level == "immutable")
            && text.to_ascii_lowercase().contains("should")
        {
            findings.push(finding(
                "DOCENF-1.6",
                &rel,
                line_of(&text, "should"),
                format!("{rel} documents immutable rules with advisory should language"),
                None,
            )?);
        }
        if text.to_ascii_lowercase().contains("rust-rules")
            && !text.to_ascii_lowercase().contains("compatibility alias")
        {
            findings.push(finding(
                "DOCENF-1.7",
                &rel,
                line_of(&text, "rust-rules"),
                format!("{rel} refers to rust-rules without saying it is a compatibility alias"),
                None,
            )?);
        }
        if text.to_ascii_lowercase().contains("rust-only")
            || text.to_ascii_lowercase().contains("rust only")
            || text
                .to_ascii_lowercase()
                .contains("typescript/python later")
            || text
                .to_ascii_lowercase()
                .contains("python/typescript later")
        {
            findings.push(finding(
                "DOCENF-1.8",
                &rel,
                1,
                format!(
                    "{rel} contains stale single-language positioning despite multi-language rules"
                ),
                None,
            )?);
        }
        if docs.iter().any(|r| r.lock_level == "advisory")
            && ![
                "promote", "profile", "failon", "severity", "warning", "error",
            ]
            .iter()
            .any(|term| text.to_ascii_lowercase().contains(term))
        {
            findings.push(finding("DOCENF-1.9", &rel, 1, format!("{rel} covers advisory rules but does not explain profile promotion or severity handling"), None)?);
        }
        if docs.iter().any(|r| {
            r.validator == "review" || r.validator == "proof" || r.family.contains("proof")
        }) && !["proof", "checklist", "review evidence", "evidence"]
            .iter()
            .any(|term| text.to_ascii_lowercase().contains(term))
        {
            findings.push(finding(
                "DOCENF-1.10",
                &rel,
                1,
                format!("{rel} covers review/proof rules but does not name the expected evidence"),
                None,
            )?);
        }
        findings.extend(code_block_findings(&rel, &text)?);
    }
    for rule in &catalog.rules {
        if let Some((_, anchor)) = rule.doc.split_once('#') {
            if anchor != anchor_name(anchor) {
                findings.push(finding(
                    "DOCENF-1.5",
                    "rules/rules.json",
                    1,
                    format!(
                        "{} uses unstable doc anchor #{anchor}; use #{}",
                        rule.id,
                        anchor_name(anchor)
                    ),
                    Some(rule.doc.clone()),
                )?);
            }
        }
        if rule.snippet.len() > 240 {
            findings.push(finding(
                "DOCENF-1.4",
                "rules/rules.json",
                1,
                format!("{} snippet is longer than 240 characters", rule.id),
                Some(rule.snippet.clone()),
            )?);
        }
    }
    let violations = findings
        .iter()
        .cloned()
        .filter_map(|f| Violation::try_from(f).ok())
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
fn markdown_files(dir: &Path) -> Result<Vec<std::path::PathBuf>, String> {
    let mut out = Vec::new();
    if !dir.exists() {
        return Ok(out);
    }
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_dir() {
            out.extend(markdown_files(&path)?)
        } else if path.extension().and_then(|v| v.to_str()) == Some("md") {
            out.push(path)
        }
    }
    Ok(out)
}
fn relative(root: &RepoRoot, path: &Path) -> Result<String, String> {
    path.strip_prefix(root.as_str())
        .map_err(|e| e.to_string())
        .map(|p| p.to_string_lossy().replace('\\', "/"))
}
fn has_heading(text: &str, heading: &str) -> bool {
    text.lines().any(|line| {
        line.trim_start_matches('#')
            .trim()
            .eq_ignore_ascii_case(heading)
    })
}
fn has_fail_pass_code(text: &str) -> bool {
    section_has_code(text, "Fails") && section_has_code(text, "Passes")
}
fn section_has_code(text: &str, heading: &str) -> bool {
    let mut active = false;
    for line in text.lines() {
        if line.trim_start().starts_with('#') {
            if active {
                return false;
            }
            active = line
                .trim_start_matches('#')
                .trim()
                .eq_ignore_ascii_case(heading)
        } else if active && line.trim_start().starts_with("```") {
            return true;
        }
    }
    false
}
fn anchor_name(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
fn line_of(text: &str, needle: &str) -> usize {
    text.lines()
        .position(|line| line.to_ascii_lowercase().contains(needle))
        .map_or(1, |n| n + 1)
}
fn code_block_findings(rel: &str, text: &str) -> Result<Vec<Finding>, String> {
    let mut out = Vec::new();
    let mut lang = "";
    let mut start = 0;
    let mut block = String::new();
    for (n, line) in text.lines().enumerate() {
        if let Some(value) = line.strip_prefix("```") {
            if lang.is_empty() {
                lang = value.trim();
                start = n + 1;
                block.clear()
            } else {
                if lang.eq_ignore_ascii_case("json")
                    && serde_json::from_str::<serde_json::Value>(&block).is_err()
                {
                    out.push(finding(
                        "DOCENF-1.3",
                        rel,
                        start,
                        "JSON code block is not parseable".to_owned(),
                        None,
                    )?)
                }
                if [
                    "js",
                    "javascript",
                    "ts",
                    "typescript",
                    "tsx",
                    "rust",
                    "rs",
                    "python",
                    "py",
                ]
                .contains(&lang.to_ascii_lowercase().as_str())
                    && !balanced(&block, lang)
                {
                    out.push(finding(
                        "DOCENF-1.3",
                        rel,
                        start,
                        format!("{lang} code block has unbalanced delimiters"),
                        None,
                    )?)
                }
                lang = ""
            }
        } else if !lang.is_empty() {
            block.push_str(line);
            block.push('\n')
        }
    }
    Ok(out)
}
fn balanced(value: &str, language: &str) -> bool {
    let mut stack = Vec::new();
    let mut chars = value.chars().peekable();
    let mut quote = None;
    let mut escaped = false;
    let mut block_comment = false;
    let python = matches!(language.to_ascii_lowercase().as_str(), "python" | "py");
    while let Some(c) = chars.next() {
        if block_comment {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                block_comment = false;
            }
            continue;
        }
        if let Some(delimiter) = quote {
            if c == '\\' && !escaped {
                escaped = true;
                continue;
            }
            if c == delimiter && !escaped {
                quote = None;
            }
            escaped = false;
            continue;
        }
        if c == '/' && chars.peek() == Some(&'/') {
            for rest in chars.by_ref() {
                if rest == '\n' {
                    break;
                }
            }
            continue;
        }
        if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            block_comment = true;
            continue;
        }
        if python && c == '#' {
            for rest in chars.by_ref() {
                if rest == '\n' {
                    break;
                }
            }
            continue;
        }
        if matches!(c, '\'' | '"' | '`') {
            quote = Some(c);
            continue;
        }
        match c {
            '{' | '(' | '[' => stack.push(c),
            '}' if stack.pop() != Some('{') => return false,
            ')' if stack.pop() != Some('(') => return false,
            ']' if stack.pop() != Some('[') => return false,
            '}' | ')' | ']' => {}
            _ => {}
        }
    }
    stack.is_empty() && quote.is_none() && !block_comment
}
fn finding(
    rule: &str,
    file: &str,
    line: usize,
    detail: String,
    snippet: Option<String>,
) -> Result<Finding, String> {
    Ok(Finding {
        rule_id: rule.parse::<RuleId>().map_err(|e| e.to_string())?,
        severity: Severity::Error,
        title: FindingTitle::new("documentation completeness violation".to_owned())
            .map_err(|e| e.to_string())?,
        detail: FindingDetail::new(detail).map_err(|e| e.to_string())?,
        snippet: snippet.and_then(|s| FindingSnippet::new(s).ok()),
        file: file.parse::<RelPath>().map_err(|e| e.to_string())?,
        line: FindingLine::known(SourceLine::try_new(
            NonZeroU32::new(u32::try_from(line).map_err(|_overflow| "line overflow")?)
                .ok_or("line overflow")?,
        )),
    })
}

#[cfg(test)]
mod tests {
    use super::check;
    use enforcer_domain::findings::ScanScope;
    use enforcer_domain::paths::RepoRoot;

    fn seed(
        root: &std::path::Path,
        markdown: &str,
    ) -> Result<RepoRoot, Box<dyn std::error::Error>> {
        std::fs::create_dir_all(root.join("rules/common"))?;
        std::fs::write(
            root.join("rules/rules.json"),
            r#"{"rules":[{"id":"RR-1.1","language":"rust","family":"source","lockLevel":"immutable","validator":"rust/test","doc":"rules/common/rule.md#covered-rules","snippet":"short"}]}"#,
        )?;
        std::fs::write(root.join("rules/common/rule.md"), markdown)?;
        Ok(root.to_string_lossy().parse()?)
    }

    #[test]
    fn rejects_missing_required_sections() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = seed(temp.path(), "# Rule\n")?;
        let report = check(&root, ScanScope::Workspace)?;
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.rule_id.as_str() == "DOCENF-1.1"));
        Ok(())
    }

    #[test]
    fn accepts_complete_source_rule_document() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = seed(temp.path(), "# Rule\n## Covered Rules\n## Fails\n```rust\nfn bad() {}\n```\n## Passes\n```rust\nfn good() {}\n```\n## Fix Recipe\n## Validator\n")?;
        assert!(check(&root, ScanScope::Workspace)?.findings.is_empty());
        Ok(())
    }

    #[test]
    fn accepts_delimiters_inside_strings_and_comments() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = seed(temp.path(), "# Rule\n## Covered Rules\n## Fails\n```rust\nfn bad() { println!(\"{\"); /* ] */ }\n```\n## Passes\n```typescript\nfunction good() { const value = \"]\"; // }\n}\n```\n## Fix Recipe\n## Validator\n")?;
        assert!(check(&root, ScanScope::Workspace)?.findings.is_empty());
        Ok(())
    }
}
