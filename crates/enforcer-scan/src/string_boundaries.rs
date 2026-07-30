//! Native Rust source-boundary checks for the `no-naked-domain-strings` route.
//!
//! This module deliberately consumes the typed effective configuration rather
//! than duplicating ownership exceptions at every caller.  It is the Rust-only
//! implementation of the frozen route's RR-6.x surface; callers must report
//! unsupported language families explicitly instead of silently treating this
//! as cross-language parity.

use regex::Regex;
use std::num::NonZeroU32;

use enforcer_domain::config_types::{EffectiveConfig, RuleEnabled};
use enforcer_domain::findings::{
    Finding, FindingDetail, FindingLine, FindingSnippet, FindingTitle, Report, ReportOutcome,
    Violation,
};
use enforcer_domain::paths::RelPath;
use enforcer_domain::scan_types::ResolvedScope;
use enforcer_domain::severity::Severity;
use enforcer_domain::telemetry_types::SourceLine;

/// Run the native Rust subset of `no-naked-domain-strings`.
///
/// Generated output and configured raw-string owners are classification
/// exclusions, not suppressions: neither is product runtime code subject to
/// this domain-boundary policy.
pub fn check(
    scope: &ResolvedScope,
    files: &[RelPath],
    config: &EffectiveConfig,
) -> Result<Report, String> {
    let mut findings = Vec::new();
    for file in files.iter().filter(|file| file.as_str().ends_with(".rs")) {
        if is_generated(file.as_str())
            || matches_any(
                &config.shape_ownership.raw_string_owner_globs,
                file.as_str(),
            )
        {
            continue;
        }
        let path = std::path::Path::new(scope.repo_root.as_str()).join(file.as_str());
        let source = std::fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", file.as_str()))?;
        findings.extend(rr_6_1(file, &source)?);
        findings.extend(rr_6_5(file, &source)?);
        if config
            .runtime_literal_policy
            .enforce_runtime_string_literals
            == RuleEnabled::Enabled
            && !is_test_surface(file.as_str())
        {
            findings.extend(rr_18_16(file, &source, config)?);
        }
    }
    findings.sort_by(|left, right| {
        (&left.file, left.line, &left.rule_id).cmp(&(&right.file, right.line, &right.rule_id))
    });
    let violations: Vec<Violation> = findings
        .iter()
        .cloned()
        .filter_map(|finding| Violation::try_from(finding).ok())
        .collect();
    Ok(Report {
        ok: if violations.is_empty() {
            ReportOutcome::Clean
        } else {
            ReportOutcome::Violations
        },
        scope: scope.kind,
        violations,
        warnings: Vec::new(),
        waived: Vec::new(),
        findings,
    })
}

fn rr_6_1(file: &RelPath, source: &str) -> Result<Vec<Finding>, String> {
    let mut findings = Vec::new();
    for (index, line) in source.lines().enumerate() {
        let declaration = line.trim_start();
        if !declaration.starts_with("fn ")
            && !declaration.starts_with("pub fn ")
            && !declaration.starts_with("pub(crate) fn ")
        {
            continue;
        }
        if contains_raw_signature_type(declaration) {
            findings.push(finding(
                "RR-6.1",
                file.clone(),
                index + 1,
                "Raw string/path type found in function signature.",
                line,
            )?);
        }
    }
    Ok(findings)
}

fn rr_18_16(
    file: &RelPath,
    source: &str,
    config: &EffectiveConfig,
) -> Result<Vec<Finding>, String> {
    let allow = config
        .runtime_literal_policy
        .runtime_string_line_allow_patterns
        .iter()
        .map(|pattern| {
            Regex::new(pattern.as_str()).map_err(|error| {
                format!(
                    "invalid runtime string allow pattern `{}`: {error}",
                    pattern.as_str()
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut findings = Vec::new();
    let mut in_cfg_test = false;
    for (index, line) in source.lines().enumerate() {
        if line.contains("#[cfg(test)]") {
            in_cfg_test = true;
            continue;
        }
        if in_cfg_test && line.trim_start().starts_with('}') {
            in_cfg_test = false;
        }
        if !in_cfg_test
            && has_string_literal(line)
            && !allow.iter().any(|pattern| pattern.is_match(line))
        {
            findings.push(finding(
                "RR-18.16",
                file.clone(),
                index + 1,
                "Runtime Rust source contains an inline string literal.",
                line,
            )?);
        }
    }
    Ok(findings)
}

fn rr_6_5(file: &RelPath, source: &str) -> Result<Vec<Finding>, String> {
    let mut findings = Vec::new();
    for (index, line) in source.lines().enumerate() {
        let Some((_, alias)) = line.split_once("type ") else {
            continue;
        };
        let Some((_, value)) = alias.split_once('=') else {
            continue;
        };
        if [
            "String", "str", "PathBuf", "Path", "usize", "u64", "u32", "i64", "i32", "bool",
        ]
        .iter()
        .any(|needle| contains_type_token(value, needle))
        {
            findings.push(finding(
                "RR-6.5",
                file.clone(),
                index + 1,
                "Raw primitive/string type alias found.",
                line,
            )?);
        }
    }
    Ok(findings)
}

fn contains_raw_signature_type(line: &str) -> bool {
    let Some(open) = line.find('(') else {
        return false;
    };
    let close = line.rfind(')').unwrap_or(line.len().saturating_sub(1));
    let Some(signature) = line.get(open..=close) else {
        return false;
    };
    ["String", "&str", "PathBuf", "&Path"]
        .iter()
        .any(|needle| contains_type_token(signature, needle))
        || line.split_once("->").is_some_and(|(_, return_type)| {
            ["String", "&str", "PathBuf", "&Path"]
                .iter()
                .any(|needle| contains_type_token(return_type, needle))
        })
}

fn contains_type_token(value: &str, needle: &str) -> bool {
    value.match_indices(needle).any(|(index, _)| {
        let before = value
            .get(..index)
            .and_then(|prefix| prefix.chars().next_back());
        let after = value
            .get(index.saturating_add(needle.len())..)
            .and_then(|suffix| suffix.chars().next());
        !before.is_some_and(|character| character.is_alphanumeric() || character == '_')
            && !after.is_some_and(|character| character.is_alphanumeric() || character == '_')
    })
}

fn has_string_literal(line: &str) -> bool {
    let mut escaped = false;
    let mut quote_count = 0_usize;
    for character in line.chars() {
        if character == '\\' && !escaped {
            escaped = true;
            continue;
        }
        if character == '"' && !escaped {
            quote_count += 1;
        }
        escaped = false;
    }
    quote_count >= 2
}

fn is_generated(path: &str) -> bool {
    path.replace('\\', "/")
        .split('/')
        .any(|segment| segment == "generated")
}

fn is_test_surface(path: &str) -> bool {
    let path = path.replace('\\', "/");
    path.contains("/tests/")
        || path.starts_with("tests/")
        || path.ends_with("_tests.rs")
        || path.ends_with("_test.rs")
}

fn matches_any(patterns: &[enforcer_domain::config_types::Glob], path: &str) -> bool {
    patterns
        .iter()
        .any(|pattern| glob_matches(pattern.as_str(), path))
}

fn glob_matches(pattern: &str, path: &str) -> bool {
    fn matches(pattern: &[u8], path: &[u8]) -> bool {
        let Some((&head, tail)) = pattern.split_first() else {
            return path.is_empty();
        };
        if head == b'*' {
            let (double_star, rest) = match tail.split_first() {
                Some((b'*', rest)) => (true, rest),
                _ => (false, tail),
            };
            return matches(rest, path)
                || path.split_first().is_some_and(|(&path_head, path_tail)| {
                    (double_star || path_head != b'/') && matches(pattern, path_tail)
                });
        }
        path.split_first()
            .is_some_and(|(&path_head, path_tail)| head == path_head && matches(tail, path_tail))
    }
    matches(
        pattern.replace('\\', "/").as_bytes(),
        path.replace('\\', "/").as_bytes(),
    )
}

fn finding(
    rule: &str,
    file: RelPath,
    line: usize,
    title: &str,
    snippet: &str,
) -> Result<Finding, String> {
    Ok(Finding {
        rule_id: rule.parse().map_err(
            |error: enforcer_domain::boundary::decode_error::DecodeError| error.to_string(),
        )?,
        severity: Severity::Error,
        title: FindingTitle::new(title.to_owned()).map_err(|error| error.to_string())?,
        detail: FindingDetail::new(
            "Replace the raw signature type with a canonical domain value.".to_owned(),
        )
        .map_err(|error| error.to_string())?,
        file,
        line: FindingLine::known(SourceLine::try_new(
            NonZeroU32::new(u32::try_from(line).map_err(|_| "line overflow")?)
                .ok_or("line overflow")?,
        )),
        snippet: FindingSnippet::new(snippet.trim().to_owned()).ok(),
    })
}

#[cfg(test)]
mod tests {
    use super::{finding, has_string_literal, rr_6_1, rr_6_5};
    use enforcer_domain::paths::RelPath;

    #[test]
    fn rr_6_1_rejects_raw_string_signature() -> Result<(), Box<dyn std::error::Error>> {
        let file: RelPath = "crates/example/src/service.rs".parse()?;
        let findings = rr_6_1(
            &file,
            "pub fn load(id: String) -> Result<(), String> { Ok(()) }",
        )?;
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id.as_str(), "RR-6.1");
        Ok(())
    }

    #[test]
    fn rr_6_1_does_not_treat_identifiers_as_types() -> Result<(), Box<dyn std::error::Error>> {
        let file: RelPath = "crates/example/src/service.rs".parse()?;
        assert!(rr_6_1(&file, "fn string_builder() { }")?.is_empty());
        Ok(())
    }

    #[test]
    fn rr_6_5_rejects_raw_aliases() -> Result<(), Box<dyn std::error::Error>> {
        let file: RelPath = "crates/example/src/types.rs".parse()?;
        let raw = "String";
        let fixture = format!("pub type AccountName = {raw};");
        let findings = rr_6_5(&file, &fixture)?;
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id.as_str(), "RR-6.5");
        Ok(())
    }

    #[test]
    fn literal_classifier_requires_a_closed_literal() {
        assert!(has_string_literal("let value = \"runtime\";"));
        assert!(!has_string_literal("let value = quoted;"));
    }

    #[test]
    fn finding_rejects_line_numbers_outside_the_domain_range(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let file: RelPath = "crates/example/src/service.rs".parse()?;
        assert!(finding("RR-6.1", file, usize::MAX, "title", "source").is_err());
        Ok(())
    }
}
