//! Config-driven source-shape policy.
//!
//! The frozen source-shape checker is deliberately lexical: it counts its
//! language constructs from source lines and brace/indent boundaries rather
//! than claiming AST precision.  This native implementation keeps that same
//! contract, including ordered exact-path and glob override application.

use std::num::NonZeroU32;

use enforcer_domain::config_types::{
    EffectiveConfig, SourceShapeKind, SourceShapeOverride, SourceShapePolicy,
};
use enforcer_domain::findings::{
    Finding, FindingDetail, FindingLine, FindingSnippet, FindingTitle, Report, ReportOutcome,
    ScanScope, Violation,
};
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::{RelPath, RepoRoot};
use enforcer_domain::severity::Severity;
use enforcer_domain::telemetry_types::SourceLine;

#[derive(Clone, Copy)]
struct Limits {
    classes: Option<usize>,
    exports: Option<usize>,
    functions: Option<usize>,
    function_lines: usize,
    lines: usize,
    types: Option<usize>,
    nesting: usize,
    branches: usize,
}

impl Limits {
    fn from_policy(policy: &SourceShapePolicy) -> Self {
        let default = match policy.kind {
            SourceShapeKind::Rust => (Some(18), None, Some(24), 1000),
            SourceShapeKind::Python => (Some(30), Some(4), None, 800),
            _ => (None, Some(1), None, 1000),
        };
        Self {
            classes: policy
                .max_classes
                .map(std::num::NonZeroUsize::get)
                .or(default.1),
            exports: policy.max_exports.map(std::num::NonZeroUsize::get).or(
                if matches!(
                    policy.kind,
                    SourceShapeKind::Typescript | SourceShapeKind::Common
                ) {
                    Some(35)
                } else {
                    None
                },
            ),
            functions: policy
                .max_functions
                .map(std::num::NonZeroUsize::get)
                .or(default.0),
            function_lines: policy
                .max_function_lines
                .map(std::num::NonZeroUsize::get)
                .unwrap_or(80),
            lines: policy
                .max_lines
                .map(std::num::NonZeroUsize::get)
                .unwrap_or(default.3),
            types: policy
                .max_types
                .map(std::num::NonZeroUsize::get)
                .or(default.2),
            nesting: policy
                .max_nesting_depth
                .map(std::num::NonZeroUsize::get)
                .unwrap_or(4),
            branches: policy
                .max_branches
                .map(std::num::NonZeroUsize::get)
                .unwrap_or(12),
        }
    }

    fn apply(&mut self, value: &SourceShapeOverride) {
        macro_rules! replace {
            ($field:ident, $value:ident) => {
                if let Some(limit) = value.$value {
                    self.$field = Some(limit.get());
                }
            };
        }
        replace!(classes, max_classes);
        replace!(exports, max_exports);
        replace!(functions, max_functions);
        if let Some(limit) = value.max_function_lines {
            self.function_lines = limit.get();
        }
        if let Some(limit) = value.max_lines {
            self.lines = limit.get();
        }
        replace!(types, max_types);
        if let Some(limit) = value.max_nesting_depth {
            self.nesting = limit.get();
        }
        if let Some(limit) = value.max_branches {
            self.branches = limit.get();
        }
    }
}

/// Execute source-shape policy only over an already-resolved scope.
pub fn check(
    root: &RepoRoot,
    scope: ScanScope,
    files: &[RelPath],
    config: &EffectiveConfig,
) -> Result<Report, String> {
    let mut findings = Vec::new();
    for policy in &config.source_shape_policies {
        for path in files.iter().filter(|path| policy_matches(policy, path)) {
            let absolute = std::path::Path::new(root.as_str()).join(path.as_str());
            let source = std::fs::read_to_string(&absolute).map_err(|error| {
                format!(
                    "cannot read source-shape file {}: {error}",
                    absolute.display()
                )
            })?;
            let mut limits = Limits::from_policy(policy);
            for override_ in config
                .source_shape_overrides
                .iter()
                .filter(|value| override_matches(value, path.as_str()))
            {
                limits.apply(override_);
            }
            findings.extend(inspect(path, &source, policy.kind, limits));
        }
    }
    findings.sort_by(|left, right| {
        left.file
            .as_str()
            .cmp(right.file.as_str())
            .then(left.line.cmp(&right.line))
            .then(left.rule_id.cmp(&right.rule_id))
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

fn policy_matches(policy: &SourceShapePolicy, path: &RelPath) -> bool {
    let path_text = path.as_str();
    policy.roots.iter().any(|root| {
        path_text == root.as_str() || path_text.starts_with(&format!("{}/", root.as_str()))
    }) && policy
        .extensions
        .iter()
        .any(|extension| path_text.ends_with(extension.as_str()))
}

fn override_matches(value: &SourceShapeOverride, path: &str) -> bool {
    value
        .path
        .as_ref()
        .is_some_and(|candidate| candidate.as_str() == path)
        || value
            .paths
            .iter()
            .any(|candidate| candidate.as_str() == path)
        || value
            .glob
            .as_ref()
            .is_some_and(|pattern| glob_matches(pattern.as_str(), path))
        || value
            .globs
            .iter()
            .any(|pattern| glob_matches(pattern.as_str(), path))
}

fn glob_matches(pattern: &str, path: &str) -> bool {
    fn inner(pattern: &[u8], path: &[u8]) -> bool {
        let Some((&head, tail)) = pattern.split_first() else {
            return path.is_empty();
        };
        if head == b'*' {
            let (deep, rest) = match tail.split_first() {
                Some((b'*', rest)) => (true, rest),
                _ => (false, tail),
            };
            return inner(rest, path)
                || path.split_first().is_some_and(|(&path_head, path_tail)| {
                    (deep || path_head != b'/') && inner(pattern, path_tail)
                });
        }
        if head == b'?' {
            return path.split_first().is_some_and(|(&path_head, path_tail)| {
                path_head != b'/' && inner(tail, path_tail)
            });
        }
        path.split_first()
            .is_some_and(|(&path_head, path_tail)| head == path_head && inner(tail, path_tail))
    }
    inner(
        pattern.replace('\\', "/").as_bytes(),
        path.replace('\\', "/").as_bytes(),
    )
}

fn inspect(path: &RelPath, source: &str, kind: SourceShapeKind, limits: Limits) -> Vec<Finding> {
    let lines = source.lines().collect::<Vec<_>>();
    let mut out = Vec::new();
    let starts = function_starts(&lines, kind);
    let metrics = Metrics::from_lines(&lines, kind, &starts);
    push_if_over(
        &mut out,
        path,
        1,
        "SRC-2.6",
        metrics.nesting,
        limits.nesting,
        "file nesting depth",
    );
    push_if_over(
        &mut out,
        path,
        1,
        "SRC-2.7",
        metrics.branches,
        limits.branches,
        "file branch points",
    );
    if let Some(limit) = limits.functions {
        push_if_over(
            &mut out,
            path,
            1,
            "SRC-1.1",
            starts.len(),
            limit,
            "file functions",
        );
    }
    if let Some(limit) = limits.classes {
        if metrics.classes > limit {
            push_dual(
                &mut out,
                path,
                1,
                metrics.classes,
                limit,
                "file classes",
                "SRC-1.1",
                "SRC-2.5",
            );
        }
    }
    if let Some(limit) = limits.exports {
        push_if_over(
            &mut out,
            path,
            1,
            "SRC-1.1",
            metrics.exports,
            limit,
            "file exports",
        );
    }
    if let Some(limit) = limits.types {
        if metrics.types > limit {
            push_dual(
                &mut out,
                path,
                1,
                metrics.types,
                limit,
                "file structs/enums",
                "SRC-1.1",
                "SRC-2.4",
            );
        }
    }
    for start in starts {
        let span = block_end(&lines, start, kind) - start + 1;
        if span > limits.function_lines {
            push_dual(
                &mut out,
                path,
                start + 1,
                span,
                limits.function_lines,
                "function lines",
                "SRC-1.1",
                "SRC-2.2",
            );
        }
    }
    if lines.len() > limits.lines {
        push_dual(
            &mut out,
            path,
            limits.lines + 1,
            lines.len(),
            limits.lines,
            "file lines",
            "SRC-1.1",
            "SRC-2.1",
        );
    }
    out
}

struct Metrics {
    nesting: usize,
    branches: usize,
    classes: usize,
    exports: usize,
    types: usize,
}
impl Metrics {
    fn from_lines(lines: &[&str], kind: SourceShapeKind, _functions: &[usize]) -> Self {
        let nesting = if matches!(kind, SourceShapeKind::Python) {
            python_nesting(lines)
        } else {
            brace_nesting(lines)
        };
        let mut metrics = Self {
            nesting,
            branches: 0,
            classes: 0,
            exports: 0,
            types: 0,
        };
        for line in lines {
            let trim = line.trim_start();
            metrics.branches += branch_count(trim, kind);
            if matches!(kind, SourceShapeKind::Python) && trim.starts_with("class ") {
                metrics.classes += 1;
            }
            if matches!(kind, SourceShapeKind::Typescript | SourceShapeKind::Common) {
                metrics.classes +=
                    usize::from(trim.starts_with("class ") || trim.starts_with("export class "));
                metrics.exports += usize::from(trim.starts_with("export "));
            }
            if matches!(kind, SourceShapeKind::Rust)
                && (trim.starts_with("struct ")
                    || trim.starts_with("enum ")
                    || trim.starts_with("pub struct ")
                    || trim.starts_with("pub enum "))
            {
                metrics.types += 1;
            }
        }
        metrics
    }
}

fn function_starts(lines: &[&str], kind: SourceShapeKind) -> Vec<usize> {
    lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            let trim = line.trim_start();
            let yes = match kind {
                SourceShapeKind::Rust => {
                    trim.starts_with("fn ")
                        || trim.starts_with("pub fn ")
                        || trim.starts_with("async fn ")
                        || trim.starts_with("pub async fn ")
                }
                SourceShapeKind::Python => {
                    trim.starts_with("def ") || trim.starts_with("async def ")
                }
                _ => {
                    trim.starts_with("function ")
                        || trim.starts_with("export function ")
                        || trim.contains(" =>")
                }
            };
            yes.then_some(index)
        })
        .collect()
}
fn branch_count(line: &str, kind: SourceShapeKind) -> usize {
    let words: &[&str] = match kind {
        SourceShapeKind::Python => &[
            "if ", "elif ", "for ", "while ", "try", "except", "with ", "match ", "case ",
        ],
        _ => &["if ", "for ", "while ", "loop", "match", "=>"],
    };
    words
        .iter()
        .map(|word| line.match_indices(word).count())
        .sum()
}
fn brace_nesting(lines: &[&str]) -> usize {
    let mut depth: usize = 0;
    let mut max: usize = 0;
    for line in lines {
        for byte in line.bytes() {
            match byte {
                b'{' => {
                    depth += 1;
                    max = max.max(depth);
                }
                b'}' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
    }
    max
}
fn python_nesting(lines: &[&str]) -> usize {
    lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .map(|spaces| spaces / 4)
        .max()
        .unwrap_or(0)
}
fn block_end(lines: &[&str], start: usize, kind: SourceShapeKind) -> usize {
    if matches!(kind, SourceShapeKind::Python) {
        let Some(start_line) = lines.get(start) else {
            return start;
        };
        let indent = start_line.len() - start_line.trim_start().len();
        return lines
            .iter()
            .enumerate()
            .skip(start + 1)
            .find(|(_, line)| {
                !line.trim().is_empty() && line.len() - line.trim_start().len() <= indent
            })
            .map_or(lines.len().saturating_sub(1), |(index, _)| {
                index.saturating_sub(1)
            });
    }
    let mut depth: usize = 0;
    let mut opened = false;
    for (index, line) in lines.iter().enumerate().skip(start) {
        for byte in line.bytes() {
            if byte == b'{' {
                depth += 1;
                opened = true;
            } else if byte == b'}' && opened {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return index;
                }
            }
        }
    }
    start
}
fn push_if_over(
    out: &mut Vec<Finding>,
    path: &RelPath,
    line: usize,
    rule: &str,
    actual: usize,
    maximum: usize,
    label: &str,
) {
    if actual > maximum {
        push(out, path, line, rule, actual, maximum, label);
    }
}
fn push_dual(
    out: &mut Vec<Finding>,
    path: &RelPath,
    line: usize,
    actual: usize,
    maximum: usize,
    label: &str,
    first: &str,
    second: &str,
) {
    push_if_over(out, path, line, first, actual, maximum, label);
    push_if_over(out, path, line, second, actual, maximum, label);
}
fn push(
    out: &mut Vec<Finding>,
    path: &RelPath,
    line: usize,
    rule: &str,
    actual: usize,
    maximum: usize,
    label: &str,
) {
    let Ok(rule_id) = rule.parse::<RuleId>() else {
        return;
    };
    let Ok(line) = u32::try_from(line) else {
        return;
    };
    let Some(line) = NonZeroU32::new(line).map(SourceLine::try_new) else {
        return;
    };
    let Ok(title) = FindingTitle::new("source-shape policy limit exceeded".to_owned()) else {
        return;
    };
    let Ok(detail) = FindingDetail::new(format!("{label} is {actual}; maximum is {maximum}"))
    else {
        return;
    };
    out.push(Finding {
        rule_id,
        severity: Severity::Error,
        title,
        detail,
        snippet: FindingSnippet::new(label.to_owned()).ok(),
        file: path.clone(),
        line: FindingLine::known(line),
    });
}

#[cfg(test)]
mod tests {
    use super::{check, glob_matches};
    use enforcer_config::load_project_config;
    use enforcer_domain::findings::{ReportOutcome, ScanScope};
    use enforcer_domain::paths::RepoRoot;

    #[test]
    fn glob_semantics_distinguish_segment_and_recursive_wildcards() {
        assert!(glob_matches("src/**/x.rs", "src/a/x.rs"));
        assert!(!glob_matches("src/*.rs", "src/a/x.rs"));
    }
    #[test]
    fn config_driven_override_changes_only_the_selected_file(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        std::fs::create_dir_all(temp.path().join("src"))?;
        std::fs::write(temp.path().join("src/a.rs"), "fn a() {}\nfn b() {}\n")?;
        std::fs::write(temp.path().join("src/b.rs"), "fn a() {}\nfn b() {}\n")?;
        std::fs::write(
            temp.path().join("config.json"),
            r#"{"schemaVersion":2,"profileName":"default","sourceShapePolicies":[{"roots":["src"],"extensions":[".rs"],"kind":"rust","maxFunctions":1}],"sourceShapeOverrides":[{"path":"src/a.rs","maxFunctions":2}]}"#,
        )?;
        let config = load_project_config(&temp.path().join("config.json"))?;
        let root: RepoRoot = temp.path().to_string_lossy().parse()?;
        let files = vec!["src/a.rs".parse()?, "src/b.rs".parse()?];
        let report = check(&root, ScanScope::Files, &files, &config)?;
        assert_eq!(report.ok, ReportOutcome::Violations);
        assert!(report
            .findings
            .iter()
            .all(|finding| finding.file.as_str() == "src/b.rs"));
        Ok(())
    }
    #[test]
    fn lexical_depth_and_branch_limits_produce_the_frozen_rule_ids(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        std::fs::create_dir_all(temp.path().join("src"))?;
        std::fs::write(
            temp.path().join("src/a.rs"),
            "fn a() { if yes { if again { } } }\n",
        )?;
        std::fs::write(
            temp.path().join("config.json"),
            r#"{"schemaVersion":2,"profileName":"default","sourceShapePolicies":[{"roots":["src"],"extensions":[".rs"],"kind":"rust","maxNestingDepth":1,"maxBranches":1}]}"#,
        )?;
        let config = load_project_config(&temp.path().join("config.json"))?;
        let root: RepoRoot = temp.path().to_string_lossy().parse()?;
        let report = check(&root, ScanScope::Files, &["src/a.rs".parse()?], &config)?;
        let ids = report
            .findings
            .iter()
            .map(|finding| finding.rule_id.as_str())
            .collect::<Vec<_>>();
        assert!(ids.contains(&"SRC-2.6"));
        assert!(ids.contains(&"SRC-2.7"));
        Ok(())
    }
}
