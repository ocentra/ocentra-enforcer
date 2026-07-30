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
        .filter_map(|value| {
            value_text(root, owner, &value).ok().map(|text| {
                (
                    value
                        .get("name")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("value")
                        .to_owned(),
                    text,
                )
            })
        })
        .collect::<Vec<_>>();
    for mirror in &mirrors {
        let Some(path) = mirror.get("path").and_then(serde_json::Value::as_str) else {
            continue;
        };
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
                .unwrap_or("");
            if let Some((_, expected)) = values.iter().find(|(candidate, _)| candidate == name) {
                if value_text(root, path, value).ok().as_deref() != Some(expected) {
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
            if text.contains(value) {
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
        let needle = format!("const {name}");
        let line = text
            .lines()
            .find(|line| line.contains(&needle))
            .ok_or("rust const missing")?;
        return line
            .split('"')
            .nth(1)
            .map(str::to_owned)
            .ok_or("rust const must be string".to_owned());
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
        let enum_start = text
            .find(&format!("enum {enum_name}"))
            .ok_or("serde enum missing")?;
        let tail = text
            .get(enum_start..)
            .ok_or("serde enum offset is invalid")?;
        let variant_at = tail.find(variant).ok_or("serde variant missing")?;
        let before = tail
            .get(..variant_at)
            .ok_or("serde variant offset is invalid")?;
        let rename_at = before.rfind("rename").ok_or("serde rename missing")?;
        return before
            .get(rename_at..)
            .ok_or("serde rename offset is invalid")?
            .split('"')
            .nth(1)
            .map(str::to_owned)
            .ok_or("serde rename must be string".to_owned());
    }
    if let Some(object_path) = spec
        .get("sourceObjectPath")
        .and_then(serde_json::Value::as_str)
    {
        let key = object_path
            .rsplit('.')
            .next()
            .ok_or("sourceObjectPath is empty")?;
        let marker = format!("{key}:");
        let value = text
            .split(&marker)
            .nth(1)
            .ok_or("sourceObjectPath is missing")?
            .trim_start();
        return value
            .split('"')
            .nth(1)
            .map(str::to_owned)
            .ok_or("sourceObjectPath must reference a quoted string".to_owned());
    }
    Err("unsupported contract value spec".to_owned())
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
}
