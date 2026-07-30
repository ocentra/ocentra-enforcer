//! Native `CONTRACT-1.1` single-source owner/mirror validator.

use enforcer_domain::findings::{
    Finding, FindingDetail, FindingLine, FindingTitle, Report, ReportOutcome, ScanScope, Violation,
};
use enforcer_domain::paths::{RelPath, RepoRoot};
use enforcer_domain::severity::Severity;
use enforcer_domain::telemetry_types::SourceLine;
use std::num::NonZeroU32;

pub fn check(
    root: &RepoRoot,
    scope: ScanScope,
    files: &[RelPath],
    explicit: Option<&str>,
) -> Result<Report, String> {
    let config = resolve_config(root, explicit).transpose()?;
    let Some(config) = config else {
        return Ok(empty(scope));
    };
    let source =
        std::fs::read_to_string(root_path(root, &config)).map_err(|error| error.to_string())?;
    let raw: serde_json::Value =
        serde_json::from_str(&source).map_err(|error| error.to_string())?;
    let mut findings = Vec::new();
    for contract in raw
        .get("contracts")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        validate_contract(root, scope, files, contract, &mut findings)?;
    }
    if let Some(roots) = raw
        .get("requiredMirrorRoots")
        .or_else(|| raw.get("singleSourceRequiredMirrorRoots"))
        .and_then(serde_json::Value::as_array)
    {
        for root_name in roots.iter().filter_map(serde_json::Value::as_str) {
            for file in files.iter().filter(|file| {
                file.as_str().starts_with(&format!("{root_name}/"))
                    && file.as_str().ends_with(".rs")
            }) {
                let covered = raw
                    .get("contracts")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|contracts| {
                        contracts
                            .iter()
                            .any(|contract| covered_by(contract, file.as_str()))
                    });
                if !covered {
                    findings.push(finding(
                        file.clone(),
                        "missing single-source manifest coverage",
                    )?);
                }
            }
        }
    }
    Ok(report(scope, findings))
}

fn validate_contract(
    root: &RepoRoot,
    scope: ScanScope,
    files: &[RelPath],
    contract: &serde_json::Value,
    findings: &mut Vec<Finding>,
) -> Result<(), String> {
    let owner = contract
        .get("ownerPath")
        .and_then(serde_json::Value::as_str)
        .ok_or("contract ownerPath is required")?;
    let mirrors = contract
        .get("mirrors")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let allowed = std::iter::once(owner)
        .chain(
            mirrors
                .iter()
                .filter_map(|mirror| mirror.get("path").and_then(serde_json::Value::as_str)),
        )
        .chain(
            contract
                .get("allowedPaths")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str),
        )
        .collect::<Vec<_>>();
    for path in &allowed {
        if !root_path(root, path).is_file() {
            findings.push(finding(
                path.parse().map_err(
                    |error: enforcer_domain::boundary::decode_error::DecodeError| error.to_string(),
                )?,
                "contract references a missing required path",
            )?);
        }
    }
    if !root_path(root, owner).is_file() {
        return Ok(());
    }
    let values = contract
        .get("values")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|value| {
            let name = value
                .get("name")
                .and_then(serde_json::Value::as_str)
                .ok_or("contract value name is required")?;
            let text = value_text(root, owner, &value)?;
            if text.is_empty() {
                return Err(format!("{owner}: {name} must be a non-empty string"));
            }
            Ok((name.to_owned(), text))
        })
        .collect::<Result<Vec<_>, String>>()?;
    for mirror in &mirrors {
        let path = mirror
            .get("path")
            .and_then(serde_json::Value::as_str)
            .ok_or("contract mirror path is required")?;
        if !root_path(root, path).is_file() {
            continue;
        };
        for value in mirror
            .get("values")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
        {
            let name = value
                .get("name")
                .and_then(serde_json::Value::as_str)
                .ok_or("mirror contract value name is required")?;
            let (_, expected) = values
                .iter()
                .find(|(candidate, _)| candidate == name)
                .ok_or_else(|| format!("{path}: {name} does not match an owner value name"))?;
            let actual = value_text(root, path, value)?;
            if actual != *expected {
                findings.push(finding(
                    path.parse().map_err(
                        |error: enforcer_domain::boundary::decode_error::DecodeError| {
                            error.to_string()
                        },
                    )?,
                    "mirror value does not match its owner",
                )?);
            }
        }
    }
    let roots = contract
        .get("scanRoots")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    for file in files.iter().filter(|file| {
        roots
            .iter()
            .filter_map(serde_json::Value::as_str)
            .any(|scan_root| file.as_str().starts_with(&format!("{scan_root}/")))
            && !allowed.iter().any(|path| *path == file.as_str())
            && !non_blocking(file.as_str())
    }) {
        let text = std::fs::read_to_string(root_path(root, file.as_str())).unwrap_or_default();
        for (_, value) in &values {
            if contains_literal(&text, value) {
                findings.push(finding(
                    file.clone(),
                    "copied configured single-source contract value",
                )?);
                break;
            }
        }
    }
    let _ = scope;
    Ok(())
}
fn value_text(root: &RepoRoot, path: &str, spec: &serde_json::Value) -> Result<String, String> {
    let text = std::fs::read_to_string(root_path(root, path)).map_err(|error| error.to_string())?;
    if let Some(value) = spec.get("text").and_then(serde_json::Value::as_str) {
        return Ok(value.to_owned());
    }
    if let Some(name) = spec.get("rustConst").and_then(serde_json::Value::as_str) {
        return rust_const(&text, name);
    }
    if let Some(json_path) = spec.get("jsonPath").and_then(serde_json::Value::as_str) {
        let value: serde_json::Value =
            serde_json::from_str(&text).map_err(|error| error.to_string())?;
        let mut current = &value;
        for segment in json_path.split('.') {
            current = current
                .get(segment)
                .ok_or_else(|| format!("{json_path} is missing"))?;
        }
        return current
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| format!("{json_path} must be a string"));
    }
    if let Some(rename) = spec
        .get("rustSerdeRename")
        .and_then(serde_json::Value::as_str)
    {
        let (enum_name, variant) = rename
            .split_once("::")
            .ok_or("rustSerdeRename must be Enum::Variant")?;
        return rust_serde_rename(&text, enum_name, variant);
    }
    if let Some(object_path) = spec
        .get("sourceObjectPath")
        .and_then(serde_json::Value::as_str)
    {
        return source_object_path(&text, object_path);
    }
    Err("unsupported contract value spec".to_owned())
}
fn rust_const(source: &str, name: &str) -> Result<String, String> {
    for line in source.lines() {
        let line = line.trim();
        let rest = line.strip_prefix("pub ").unwrap_or(line);
        let Some(rest) = rest.strip_prefix("const ") else {
            continue;
        };
        let Some(rest) = rest.strip_prefix(name) else {
            continue;
        };
        if !rest.starts_with(':') || !rest.contains("&str") || !rest.contains('=') {
            continue;
        }
        return quoted_after(
            rest.split_once('=')
                .map_or("", |(_, value)| value)
                .trim_start(),
        )
        .ok_or_else(|| format!("{name} string const is missing"));
    }
    Err(format!("{name} string const is missing"))
}
fn rust_serde_rename(source: &str, enum_name: &str, variant: &str) -> Result<String, String> {
    let marker = format!("enum {enum_name}");
    let start = source
        .find(&marker)
        .ok_or_else(|| format!("{enum_name} enum is missing"))?;
    let body = source
        .get(start..)
        .and_then(|tail| tail.split_once('{').map(|(_, rest)| rest))
        .unwrap_or("");
    let body = body.split("\n}").next().unwrap_or(body);
    let variant_marker = format!("{variant}");
    for (index, line) in body.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with(&variant_marker)
            && trimmed
                .get(variant_marker.len()..)
                .unwrap_or("")
                .chars()
                .next()
                .is_none_or(|c| !c.is_ascii_alphanumeric() && c != '_')
        {
            let prefix = body.lines().take(index).collect::<Vec<_>>().join("\n");
            let attribute = prefix.rsplit("#[serde(rename").next().unwrap_or("");
            if let Some(value) = quoted_after(attribute) {
                return Ok(value);
            }
        }
    }
    Err(format!("{enum_name}::{variant} serde rename is missing"))
}
fn source_object_path(source: &str, path: &str) -> Result<String, String> {
    let Some(dot) = path.rfind('.') else {
        return Err(format!("{path} must be ObjectName.PropertyName"));
    };
    if dot == 0 || dot + 1 == path.len() {
        return Err(format!("{path} must be ObjectName.PropertyName"));
    }
    let (object, property_path) = path.split_at(dot);
    let property_path = property_path
        .get(1..)
        .ok_or_else(|| format!("{path} property path is invalid"))?;
    let (property, index) = match property_path.split_once('[') {
        Some((property, suffix)) if suffix.ends_with(']') => (
            property,
            Some(
                suffix
                    .get(..suffix.len().saturating_sub(1))
                    .ok_or_else(|| format!("{path} array index is invalid"))?
                    .parse::<usize>()
                    .map_err(|_| format!("{path} array index is invalid"))?,
            ),
        ),
        Some(_) => return Err(format!("{path} array index is invalid")),
        None => (property_path, None),
    };
    let object_body =
        object_body(source, object).ok_or_else(|| format!("{path} constant object is missing"))?;
    let marker = format!("{property}:");
    let Some(value) = object_body.split(&marker).nth(1) else {
        return Err(format!("{path} string literal is missing"));
    };
    let value = value.trim_start();
    if let Some(index) = index {
        let Some(array) = value
            .strip_prefix('[')
            .and_then(|tail| tail.split(']').next())
        else {
            return Err(format!("{path} array literal is missing"));
        };
        return quoted_values(array)
            .get(index)
            .cloned()
            .ok_or_else(|| format!("{path} array entry is missing"));
    }
    quoted_after(value)
        .or_else(|| parse_literal(value))
        .ok_or_else(|| format!("{path} string literal is missing"))
}
fn object_body<'a>(source: &'a str, name: &str) -> Option<&'a str> {
    let marker = format!("const {name}");
    let start = source.find(&marker)?;
    let assignment = source.get(start..)?.split_once('=')?.1;
    let body = assignment.trim_start();
    let body = if body.starts_with("defineLiteralKindGroup(") {
        body.split_once('{')?.1
    } else {
        body.strip_prefix('{')?
    };
    Some(body.split_once('}')?.0)
}
fn quoted_after(value: &str) -> Option<String> {
    let value = value.trim_start();
    let quote = value
        .chars()
        .next()
        .filter(|quote| matches!(quote, '\'' | '"' | '`'))?;
    let tail = value.get(quote.len_utf8()..)?;
    Some(tail.split(quote).next()?.to_owned())
}
fn parse_literal(value: &str) -> Option<String> {
    let marker = ".parse(";
    let inner = value.split_once(marker)?.1.trim_start();
    quoted_after(inner)
}
fn quoted_values(value: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut rest = value;
    while let Some(index) = rest.find(['\'', '"', '`']) {
        let Some(quote) = rest.get(index..).and_then(|tail| tail.chars().next()) else {
            break;
        };
        let Some(after) = rest.get(index + quote.len_utf8()..) else {
            break;
        };
        if let Some(end) = after.find(quote) {
            let Some(entry) = after.get(..end) else { break };
            let Some(next) = after.get(end + quote.len_utf8()..) else {
                break;
            };
            values.push(entry.to_owned());
            rest = next;
        } else {
            break;
        }
    }
    values
}
fn contains_literal(text: &str, value: &str) -> bool {
    let valid = |c: char| c.is_ascii_alphanumeric() || matches!(c, '@' | '.' | '_' | '/' | '-');
    text.match_indices(value).any(|(at, _)| {
        !text
            .get(..at)
            .and_then(|prefix| prefix.chars().next_back())
            .is_some_and(valid)
            && !text
                .get(at.saturating_add(value.len())..)
                .and_then(|suffix| suffix.chars().next())
                .is_some_and(valid)
    })
}
fn root_path(root: &RepoRoot, path: &str) -> std::path::PathBuf {
    std::path::Path::new(root.as_str()).join(path)
}
fn resolve_config(root: &RepoRoot, explicit: Option<&str>) -> Option<Result<String, String>> {
    explicit
        .map(str::to_owned)
        .filter(|path| root_path(root, path).is_file())
        .map(Ok)
        .or_else(|| {
            [
                "ocentra-enforcer.single-source-contracts.json",
                "scripts/check-single-source-contracts.json",
            ]
            .into_iter()
            .find(|path| root_path(root, path).is_file())
            .map(|path| Ok(path.to_owned()))
        })
}
fn covered_by(contract: &serde_json::Value, path: &str) -> bool {
    contract
        .get("ownerPath")
        .and_then(serde_json::Value::as_str)
        == Some(path)
        || contract
            .get("mirrors")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|mirrors| {
                mirrors.iter().any(|mirror| {
                    mirror.get("path").and_then(serde_json::Value::as_str) == Some(path)
                })
            })
        || contract
            .get("allowedPaths")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|paths| {
                paths
                    .iter()
                    .any(|candidate| candidate.as_str() == Some(path))
            })
}
fn non_blocking(path: &str) -> bool {
    path.contains("/tests/")
        || path.starts_with("tests/")
        || path.starts_with("docs/")
        || path.ends_with("_tests.rs")
}
fn finding(file: RelPath, detail: &str) -> Result<Finding, String> {
    Ok(Finding {
        rule_id: "CONTRACT-1.1".parse().map_err(
            |error: enforcer_domain::boundary::decode_error::DecodeError| error.to_string(),
        )?,
        severity: Severity::Error,
        title: FindingTitle::new("single-source contract violation".to_owned())
            .map_err(|error| error.to_string())?,
        detail: FindingDetail::new(detail.to_owned()).map_err(|error| error.to_string())?,
        snippet: None,
        file,
        line: FindingLine::known(SourceLine::try_new(NonZeroU32::MIN)),
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
fn empty(scope: ScanScope) -> Report {
    report(scope, Vec::new())
}

#[cfg(test)]
mod tests {
    use super::check;
    use enforcer_domain::findings::{ReportOutcome, ScanScope};
    use enforcer_domain::paths::{RelPath, RepoRoot};

    fn write(
        root: &std::path::Path,
        path: &str,
        source: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let target = root.join(path);
        std::fs::create_dir_all(target.parent().ok_or("parent")?)?;
        std::fs::write(target, source)?;
        Ok(())
    }
    fn check_config(
        root: &std::path::Path,
        config: &str,
        files: &[&str],
    ) -> Result<enforcer_domain::findings::Report, Box<dyn std::error::Error>> {
        write(root, "contracts.json", config)?;
        let repo: RepoRoot = root.to_string_lossy().parse()?;
        let paths = files
            .iter()
            .map(|path| path.parse::<RelPath>())
            .collect::<Result<Vec<_>, _>>()?;
        Ok(check(
            &repo,
            ScanScope::Files,
            &paths,
            Some("contracts.json"),
        )?)
    }

    #[test]
    fn selectors_owner_mirror_and_coverage_are_enforced() -> Result<(), Box<dyn std::error::Error>>
    {
        let temp = tempfile::tempdir()?;
        write(temp.path(),"src/owner.rs","pub const CODE: &str = \"owner\";\n#[derive(serde::Serialize)] enum Kind { #[serde(rename = \"wire\")] Variant }\n")?;
        write(
            temp.path(),
            "src/mirror.rs",
            "pub const CODE: &str = \"wrong\";\n",
        )?;
        write(temp.path(), "src/copy.rs", "let value = \"owner\";\n")?;
        write(
            temp.path(),
            "mirror/uncovered.rs",
            "pub fn uncovered() {}\n",
        )?;
        write(
            temp.path(),
            "src/value.json",
            "{\"nested\":{\"name\":\"json\"}}",
        )?;
        let config = r#"{"requiredMirrorRoots":["mirror"],"contracts":[{"name":"a","ownerPath":"src/owner.rs","scanRoots":["src"],"mirrors":[{"path":"src/mirror.rs","values":[{"name":"code","rustConst":"CODE"}]}],"values":[{"name":"code","rustConst":"CODE"},{"name":"wire","rustSerdeRename":"Kind::Variant"}]},{"name":"json","ownerPath":"src/value.json","scanRoots":[],"values":[{"name":"json","jsonPath":"nested.name"}]}]}"#;
        let report = check_config(
            temp.path(),
            config,
            &[
                "src/owner.rs",
                "src/mirror.rs",
                "src/copy.rs",
                "src/value.json",
                "mirror/uncovered.rs",
            ],
        )?;
        assert_eq!(report.ok, ReportOutcome::Violations);
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.file.as_str() == "src/mirror.rs"));
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.file.as_str() == "src/copy.rs"));
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.file.as_str() == "mirror/uncovered.rs"));
        Ok(())
    }

    #[test]
    fn source_object_path_selector_reads_quoted_value() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        write(
            temp.path(),
            "src/owner.ts",
            "export const values = { label: \"object\" };\n",
        )?;
        let config = r#"{"contracts":[{"ownerPath":"src/owner.ts","scanRoots":["src"],"values":[{"name":"label","sourceObjectPath":"values.label"}]}]}"#;
        let report = check_config(temp.path(), config, &["src/owner.ts"])?;
        assert_eq!(report.ok, ReportOutcome::Clean);
        Ok(())
    }

    #[test]
    fn each_selector_detects_a_copied_owner_value() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        write(temp.path(), "src/constants.rs", "pub const CODE: &str = \"rust\";\n#[derive(serde::Serialize)]\nenum Kind {\n    #[serde(rename = \"wire\")]\n    Variant,\n}\n")?;
        write(temp.path(), "src/objects.ts", "export const labels = { direct: \"object\", parsed: Kind.parse(\"parsed\"), entries: [\"zero\", \"one\"] } as const;\n")?;
        write(
            temp.path(),
            "src/value.json",
            "{\"nested\":{\"name\":\"json\"}}",
        )?;
        write(
            temp.path(),
            "scan/copies.rs",
            "let all = [\"rust\", \"wire\", \"object\", \"parsed\", \"one\", \"json\"];\n",
        )?;
        let config = r#"{"contracts":[{"ownerPath":"src/constants.rs","scanRoots":["scan"],"values":[{"name":"rust","rustConst":"CODE"},{"name":"wire","rustSerdeRename":"Kind::Variant"}]},{"ownerPath":"src/objects.ts","scanRoots":["scan"],"values":[{"name":"direct","sourceObjectPath":"labels.direct"},{"name":"parsed","sourceObjectPath":"labels.parsed"},{"name":"indexed","sourceObjectPath":"labels.entries[1]"}]},{"ownerPath":"src/value.json","scanRoots":["scan"],"values":[{"name":"json","jsonPath":"nested.name"}]}]}"#;
        let report = check_config(
            temp.path(),
            config,
            &[
                "src/constants.rs",
                "src/objects.ts",
                "src/value.json",
                "scan/copies.rs",
            ],
        )?;
        assert_eq!(
            report
                .findings
                .iter()
                .filter(|finding| finding.file.as_str() == "scan/copies.rs")
                .count(),
            3
        );
        Ok(())
    }

    #[test]
    fn literal_matching_respects_frozen_boundaries() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        write(
            temp.path(),
            "src/owner.rs",
            "pub const CODE: &str = \"owner\";\n",
        )?;
        write(
            temp.path(),
            "scan/safe.rs",
            "let value = \"preownerpost\";\n",
        )?;
        let config = r#"{"contracts":[{"ownerPath":"src/owner.rs","scanRoots":["scan"],"values":[{"name":"code","rustConst":"CODE"}]}]}"#;
        let report = check_config(temp.path(), config, &["src/owner.rs", "scan/safe.rs"])?;
        assert_eq!(report.ok, ReportOutcome::Clean);
        Ok(())
    }

    #[test]
    fn malformed_owner_or_mirror_value_specs_fail_closed() -> Result<(), Box<dyn std::error::Error>>
    {
        let temp = tempfile::tempdir()?;
        write(
            temp.path(),
            "src/owner.rs",
            "pub const CODE: &str = \"owner\";\n",
        )?;
        write(
            temp.path(),
            "src/mirror.rs",
            "pub const CODE: &str = \"owner\";\n",
        )?;
        let missing_selector =
            r#"{"contracts":[{"ownerPath":"src/owner.rs","values":[{"name":"code"}]}]}"#;
        let error = check_config(temp.path(), missing_selector, &["src/owner.rs"])
            .expect_err("missing owner selector must reject the contract");
        assert!(error
            .to_string()
            .contains("unsupported contract value spec"));
        let unknown_mirror_name = r#"{"contracts":[{"ownerPath":"src/owner.rs","values":[{"name":"code","rustConst":"CODE"}],"mirrors":[{"path":"src/mirror.rs","values":[{"name":"other","rustConst":"CODE"}]}]}]}"#;
        let error = check_config(
            temp.path(),
            unknown_mirror_name,
            &["src/owner.rs", "src/mirror.rs"],
        )
        .expect_err("mirror values must name an owner value");
        assert!(error
            .to_string()
            .contains("does not match an owner value name"));
        let missing_mirror_selector = r#"{"contracts":[{"ownerPath":"src/owner.rs","values":[{"name":"code","rustConst":"CODE"}],"mirrors":[{"path":"src/mirror.rs","values":[{"name":"code"}]}]}]}"#;
        let error = check_config(
            temp.path(),
            missing_mirror_selector,
            &["src/owner.rs", "src/mirror.rs"],
        )
        .expect_err("missing mirror selector must reject the contract");
        assert!(error
            .to_string()
            .contains("unsupported contract value spec"));
        Ok(())
    }
}
