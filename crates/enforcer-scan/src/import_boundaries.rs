//! Config-driven TypeScript and JavaScript import-boundary policy.

use std::num::NonZeroU32;

use enforcer_domain::config_types::{EffectiveConfig, ImportBoundaryPolicy};
use enforcer_domain::findings::{
    Finding, FindingDetail, FindingLine, FindingSnippet, FindingTitle, Report, ReportOutcome,
    ScanScope, Violation,
};
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::{RelPath, RepoRoot};
use enforcer_domain::severity::Severity;
use enforcer_domain::telemetry_types::SourceLine;

pub fn check(
    root: &RepoRoot,
    scope: ScanScope,
    files: &[RelPath],
    config: &EffectiveConfig,
) -> Result<Report, String> {
    let mut findings = Vec::new();
    for file in files.iter().filter(|file| is_script(file.as_str())) {
        let path = file.as_str();
        let source = std::fs::read_to_string(std::path::Path::new(root.as_str()).join(path))
            .map_err(|error| format!("cannot read import-boundary file {path}: {error}"))?;
        for policy in config
            .import_boundary_policies
            .iter()
            .filter(|policy| policy_matches(policy, path))
        {
            for (index, line) in source.split('\n').enumerate() {
                let line = line.strip_suffix('\r').unwrap_or(line);
                let Some(specifier) = import_specifier(line) else {
                    continue;
                };
                if policy
                    .forbidden_imports
                    .iter()
                    .any(|glob| glob_matches(glob.as_str(), specifier))
                    && !policy
                        .allowed_imports
                        .iter()
                        .any(|glob| glob_matches(glob.as_str(), specifier))
                {
                    if let Some(finding) = finding(file, index + 1, line, specifier, policy) {
                        findings.push(finding);
                    }
                }
            }
        }
    }
    findings.sort_by(|left, right| {
        (&left.file, left.line, &left.rule_id).cmp(&(&right.file, right.line, &right.rule_id))
    });
    let violations: Vec<Violation> = findings
        .iter()
        .cloned()
        .filter_map(|value| Violation::try_from(value).ok())
        .collect();
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

fn is_script(path: &str) -> bool {
    [".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".mts", ".cts"]
        .iter()
        .any(|extension| path.ends_with(extension))
}
fn policy_matches(policy: &ImportBoundaryPolicy, path: &str) -> bool {
    policy.roots.is_empty()
        || policy
            .roots
            .iter()
            .any(|root| path == root.as_str() || path.starts_with(&format!("{}/", root.as_str())))
}
fn import_specifier(line: &str) -> Option<&str> {
    let from = import_from_keyword(line)?;
    let remainder = line.get(from.checked_add("from".len())?..)?.trim_start();
    let quote = remainder.chars().next()?;
    if !matches!(quote, '\'' | '\"' | '`') {
        return None;
    }
    let content = remainder.get(quote.len_utf8()..)?;
    content.split_once(quote).map(|(value, _)| value)
}

fn import_from_keyword(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut quote = None;
    let mut escaped = false;
    let mut index = 0;

    while index < bytes.len() {
        let byte = *bytes.get(index)?;
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == active_quote {
                quote = None;
            }
            index += 1;
            continue;
        }

        if matches!(byte, b'\'' | b'"' | b'`') {
            quote = Some(byte);
            index += 1;
            continue;
        }

        let end = index.checked_add("from".len())?;
        if bytes.get(index..end) == Some(b"from")
            && index
                .checked_sub(1)
                .and_then(|previous| bytes.get(previous))
                .is_none_or(|value| !is_identifier_byte(*value))
            && bytes
                .get(end)
                .is_none_or(|value| !is_identifier_byte(*value))
        {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn is_identifier_byte(value: u8) -> bool {
    value.is_ascii_alphanumeric() || matches!(value, b'_' | b'$')
}
fn glob_matches(pattern: &str, text: &str) -> bool {
    fn inner(pattern: &[u8], text: &[u8]) -> bool {
        let Some((&head, tail)) = pattern.split_first() else {
            return text.is_empty();
        };
        if head == b'*' {
            let (deep, rest) = match tail.split_first() {
                Some((b'*', rest)) => (true, rest),
                _ => (false, tail),
            };
            return inner(rest, text)
                || text.split_first().is_some_and(|(&value, remainder)| {
                    (deep || value != b'/') && inner(pattern, remainder)
                });
        }
        if head == b'?' {
            return text
                .split_first()
                .is_some_and(|(&value, remainder)| value != b'/' && inner(tail, remainder));
        }
        text.split_first()
            .is_some_and(|(&value, remainder)| head == value && inner(tail, remainder))
    }
    inner(
        pattern.replace('\\', "/").as_bytes(),
        text.replace('\\', "/").as_bytes(),
    )
}
fn finding(
    file: &RelPath,
    line: usize,
    source: &str,
    specifier: &str,
    policy: &ImportBoundaryPolicy,
) -> Option<Finding> {
    let rule_id = "TS-4.1".parse::<RuleId>().ok()?;
    let line = u32::try_from(line)
        .ok()
        .and_then(NonZeroU32::new)
        .map(SourceLine::try_new)?;
    Some(Finding {
        rule_id,
        severity: Severity::Error,
        title: FindingTitle::new("Import boundary policy must be respected".to_owned()).ok()?,
        detail: FindingDetail::new(
            policy
                .message
                .as_ref()
                .map(|value| value.as_str().to_owned())
                .unwrap_or_else(|| format!("import \"{specifier}\" crosses a configured boundary")),
        )
        .ok()?,
        snippet: FindingSnippet::new(source.trim().to_owned()).ok(),
        file: file.clone(),
        line: FindingLine::known(line),
    })
}

#[cfg(test)]
mod tests {
    use super::check;
    use enforcer_config::load_project_config;
    use enforcer_domain::findings::{ReportOutcome, ScanScope};
    use enforcer_domain::paths::RepoRoot;
    #[test]
    fn configured_policy_rejects_forbidden_and_honours_allowed_imports(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        std::fs::create_dir_all(temp.path().join("src/domain"))?;
        std::fs::write(
            temp.path().join("src/domain/bad.ts"),
            "import x from '@infra/x';\n",
        )?;
        std::fs::write(
            temp.path().join("src/domain/good.ts"),
            "import x from '@infra/allowed/x';\n",
        )?;
        std::fs::write(
            temp.path().join("config.json"),
            r#"{"schemaVersion":2,"profileName":"default","importBoundaryPolicies":[{"roots":["src/domain"],"forbiddenImports":["@infra/**"],"allowedImports":["@infra/allowed/**"],"message":"domain may not depend on infra"}]}"#,
        )?;
        let root: RepoRoot = temp.path().to_string_lossy().parse()?;
        let config = load_project_config(&temp.path().join("config.json"))?;
        let report = check(
            &root,
            ScanScope::Files,
            &["src/domain/bad.ts".parse()?, "src/domain/good.ts".parse()?],
            &config,
        )?;
        assert_eq!(report.ok, ReportOutcome::Violations);
        assert_eq!(report.findings.len(), 1);
        assert_eq!(
            report.findings[0].detail.as_str(),
            "domain may not depend on infra"
        );
        Ok(())
    }

    #[test]
    fn import_binding_with_from_prefix_uses_the_from_keyword(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        std::fs::create_dir_all(temp.path().join("src/domain"))?;
        std::fs::write(
            temp.path().join("src/domain/bad.ts"),
            "import { fromPromise } from '@infra/client';\n",
        )?;
        std::fs::write(
            temp.path().join("config.json"),
            r#"{"schemaVersion":2,"profileName":"default","importBoundaryPolicies":[{"roots":["src/domain"],"forbiddenImports":["@infra/**"]}]}"#,
        )?;
        let root: RepoRoot = temp.path().to_string_lossy().parse()?;
        let config = load_project_config(&temp.path().join("config.json"))?;
        let report = check(
            &root,
            ScanScope::Files,
            &["src/domain/bad.ts".parse()?],
            &config,
        )?;

        assert_eq!(report.ok, ReportOutcome::Violations);
        assert_eq!(report.findings.len(), 1);
        assert!(report.findings[0].detail.as_str().contains("@infra/client"));
        Ok(())
    }

    #[test]
    fn no_configured_policy_is_clean_without_a_legacy_default(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        std::fs::create_dir_all(temp.path().join("src/domain"))?;
        std::fs::write(
            temp.path().join("src/domain/model.js"),
            "import x from '@infra/x';\n",
        )?;
        std::fs::write(
            temp.path().join("config.json"),
            r#"{"schemaVersion":2,"profileName":"default"}"#,
        )?;
        let root: RepoRoot = temp.path().to_string_lossy().parse()?;
        let config = load_project_config(&temp.path().join("config.json"))?;
        let report = check(
            &root,
            ScanScope::Files,
            &["src/domain/model.js".parse()?],
            &config,
        )?;
        assert_eq!(report.ok, ReportOutcome::Clean);
        assert!(report.findings.is_empty());
        Ok(())
    }
}
