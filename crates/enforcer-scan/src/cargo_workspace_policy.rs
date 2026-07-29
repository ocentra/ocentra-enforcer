//! Native Cargo workspace dependency policy.
//!
//! A member crate may link to another declared workspace member by `path`.
//! Intentional grammar sources may also be vendored below the member's own
//! `vendor/` subtree. Any other local path dependency is rejected because it
//! escapes the workspace's reviewed package set.

use std::collections::BTreeSet;
use std::num::NonZeroU32;

use enforcer_domain::boundary::validation::ValidationSourceText;
use enforcer_domain::findings::{
    Finding, FindingDetail, FindingLine, FindingSnippet, FindingTitle,
};
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_domain::telemetry_types::SourceLine;

/// Return one `RR-9.3` finding for each arbitrary local Cargo dependency.
#[cfg(test)]
fn findings_for_sources(sources: &[(RelPath, Option<ValidationSourceText>)]) -> Vec<Finding> {
    findings_for_sources_with_inventory(sources, sources)
}

/// Evaluate scoped manifests using a complete inventory of workspace package manifests.
pub fn findings_for_sources_with_inventory(
    sources: &[(RelPath, Option<ValidationSourceText>)],
    workspace_inventory: &[(RelPath, Option<ValidationSourceText>)],
) -> Vec<Finding> {
    let workspace_members = sources
        .iter()
        .chain(workspace_inventory)
        .filter(|(file, _)| is_cargo_manifest(file))
        .filter_map(|(file, source)| source.as_ref().and_then(|_| manifest_directory(file)))
        .collect::<BTreeSet<_>>();

    let Ok(rule_id) = "RR-9.3".parse::<RuleId>() else {
        return Vec::new();
    };
    let mut findings = Vec::new();
    for (file, source) in sources {
        if !is_cargo_manifest(file) {
            continue;
        }
        let Some(source) = source else {
            continue;
        };
        if file.as_str() == "Cargo.toml" {
            continue;
        }
        let mut section = "";
        for (offset, line) in source.as_source().as_str().lines().enumerate() {
            if let Some(header) = section_header(line) {
                section = header;
            }
            if !is_dependency_section(section) || !line.contains("path") {
                continue;
            }
            let Some(target) = dependency_target(file, line) else {
                continue;
            };
            if workspace_members.contains(&target) || is_vendor_path_dependency(line) {
                continue;
            }
            let Ok(source_line) = u32::try_from(offset + 1)
                .map_err(|_source| ())
                .and_then(|line| NonZeroU32::new(line).ok_or(()))
                .map(SourceLine::try_new)
            else {
                continue;
            };
            let Ok(title) = FindingTitle::new("path dependency is not a workspace member".into())
            else {
                continue;
            };
            let Ok(detail) = FindingDetail::new(
                "Fix: use a declared workspace member or a registry dependency; arbitrary local paths are not accepted.".into(),
            ) else {
                continue;
            };
            let Ok(snippet) = FindingSnippet::new(line.into()) else {
                continue;
            };
            findings.push(Finding {
                // CLONE-JUSTIFICATION: every independent finding owns its
                // stable rule identifier for report serialization.
                rule_id: rule_id.clone(),
                severity: Severity::Error,
                title,
                detail,
                // CLONE-JUSTIFICATION: findings outlive the borrowed source
                // inventory used to evaluate this workspace.
                file: file.clone(),
                line: FindingLine::known(source_line),
                snippet: Some(snippet),
            });
        }
    }
    findings
}

fn is_cargo_manifest(file: &RelPath) -> bool {
    file.as_str().ends_with("Cargo.toml")
}

fn manifest_directory(file: &RelPath) -> Option<String> {
    file.as_str().strip_suffix("/Cargo.toml").map(str::to_owned)
}

fn section_header(line: &str) -> Option<&str> {
    line.trim().strip_prefix('[')?.strip_suffix(']')
}

fn is_dependency_section(section: &str) -> bool {
    matches!(
        section,
        "dependencies" | "dev-dependencies" | "build-dependencies" | "workspace.dependencies"
    ) || section.starts_with("target.") && section.ends_with(".dependencies")
}

fn dependency_target(file: &RelPath, line: &str) -> Option<String> {
    let dependency_path = dependency_path(line)?.replace('\\', "/");
    if dependency_path.starts_with('/')
        || dependency_path
            .as_bytes()
            .get(1)
            .is_some_and(|separator| *separator == b':')
    {
        return Some(dependency_path);
    }
    let mut segments = manifest_directory(file)?
        .split('/')
        .map(str::to_owned)
        .collect::<Vec<_>>();
    for segment in dependency_path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                if segments.pop().is_none() {
                    return Some(dependency_path);
                }
            }
            value => segments.push(value.to_owned()),
        }
    }
    Some(segments.join("/"))
}

fn dependency_path(line: &str) -> Option<&str> {
    let (_, value) = line.split_once('=')?;
    value.split(',').find_map(|field| {
        let (key, path) = field.split_once('=')?;
        (key.trim().trim_start_matches('{').trim() == "path")
            .then(|| path.trim().trim_end_matches('}').trim().trim_matches('"'))
    })
}

fn is_vendor_path_dependency(line: &str) -> bool {
    let Some((_, value)) = line.split_once('=') else {
        return false;
    };
    value.split(',').any(|field| {
        let Some((key, path)) = field.split_once('=') else {
            return false;
        };
        if key.trim().trim_start_matches('{').trim() != "path" {
            return false;
        }
        let path = path.trim().trim_end_matches('}').trim().trim_matches('"');
        path == "vendor" || path.starts_with("vendor/") || path.starts_with(r"vendor\")
    })
}

#[cfg(test)]
mod tests {
    use super::findings_for_sources;
    use enforcer_domain::boundary::validation::ValidationSourceText;
    use enforcer_domain::findings::FindingLine;
    use enforcer_domain::paths::RelPath;
    use enforcer_domain::telemetry_types::SourceLine;
    use std::num::NonZeroU32;

    fn source(
        path: RelPath,
        contents: impl Into<ValidationSourceText>,
    ) -> Result<(RelPath, Option<ValidationSourceText>), Box<dyn std::error::Error>> {
        Ok((path, Some(contents.into())))
    }

    #[test]
    fn allows_workspace_member_paths_and_rejects_arbitrary_local_paths(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let sources = vec![
            source(
                "Cargo.toml".parse()?,
                "[workspace]\nmembers = [\"crates/core\", \"crates/app\"]\n\n[workspace.dependencies]\ncore = { path = \"crates/core\" }\n",
            )?,
            source(
                "crates/core/Cargo.toml".parse()?,
                "[package]\nname = \"core\"\n",
            )?,
            source(
                "crates/app/Cargo.toml".parse()?,
                "[package]\nname = \"app\"\n\n[dependencies]\ncore = { path = \"../core\" }\noutside = { path = \"../outside\" }\n",
            )?,
        ];
        let findings = findings_for_sources(&sources);
        let finding = findings
            .first()
            .ok_or_else(|| std::io::Error::other("expected one external path finding"))?;
        assert_eq!(finding.rule_id.as_str(), "RR-9.3");
        assert_eq!(finding.file.as_str(), "crates/app/Cargo.toml");
        assert_eq!(
            finding.line,
            FindingLine::known(SourceLine::try_new(
                NonZeroU32::new(6)
                    .ok_or_else(|| std::io::Error::other("fixture line must be positive"))?,
            ))
        );
        Ok(())
    }

    #[test]
    fn allows_manifest_relative_vendor_paths_but_rejects_external_paths(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let sources = vec![
            source(
                "Cargo.toml".parse()?,
                "[workspace]\nmembers = [\"crates/app\"]\n",
            )?,
            source(
                "crates/app/Cargo.toml".parse()?,
                "[package]\nname = \"app\"\n\n[dependencies]\nvendor-grammar = { path = \"vendor/tree-sitter-grammar\" }\noutside = { path = \"../outside\" }\n",
            )?,
        ];
        let findings = findings_for_sources(&sources);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id.as_str(), "RR-9.3");
        assert_eq!(findings[0].file.as_str(), "crates/app/Cargo.toml");
        assert_eq!(
            findings[0].line,
            FindingLine::known(SourceLine::try_new(
                NonZeroU32::new(6)
                    .ok_or_else(|| std::io::Error::other("fixture line must be positive"))?,
            ))
        );
        Ok(())
    }

    #[test]
    fn workspace_package_name_cannot_exempt_a_different_local_path(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let sources = vec![
            source(
                "Cargo.toml".parse()?,
                "[workspace]\nmembers = [\"crates/core\", \"crates/app\"]\n",
            )?,
            source(
                "crates/core/Cargo.toml".parse()?,
                "[package]\nname = \"core\"\n",
            )?,
            source(
                "crates/app/Cargo.toml".parse()?,
                "[package]\nname = \"app\"\n\n[dependencies]\ncore = { path = \"../outside\" }\n",
            )?,
        ];
        let findings = findings_for_sources(&sources);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id.as_str(), "RR-9.3");
        Ok(())
    }
}
