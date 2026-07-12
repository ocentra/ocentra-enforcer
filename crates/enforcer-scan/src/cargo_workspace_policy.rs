//! Native Cargo workspace dependency policy.
//!
//! A member crate may link to another declared workspace member by `path`.
//! Any other local path dependency is rejected because it escapes the
//! workspace's reviewed package set.

use std::collections::BTreeSet;

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;

/// Return one `RR-9.3` finding for each arbitrary local Cargo dependency.
pub fn findings_for_sources(sources: &[(RelPath, Option<String>)]) -> Vec<Finding> {
    let workspace_packages = sources
        .iter()
        .filter(|(file, _)| is_cargo_manifest(file))
        .filter_map(|(_, source)| source.as_deref())
        .filter_map(package_name)
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
        let mut section = "";
        for (offset, line) in source.lines().enumerate() {
            if let Some(header) = section_header(line) {
                section = header;
            }
            if !is_dependency_section(section) || !line.contains("path") {
                continue;
            }
            let Some(name) = dependency_name(line) else {
                continue;
            };
            if workspace_packages.contains(name) {
                continue;
            }
            findings.push(Finding {
                // CLONE-JUSTIFICATION: every independent finding owns its
                // stable rule identifier for report serialization.
                rule_id: rule_id.clone(),
                severity: Severity::Error,
                title: String::from("path dependency is not a workspace member"),
                detail: String::from(
                    "Fix: use a declared workspace member or a registry dependency; arbitrary local paths are not accepted.",
                ),
                // CLONE-JUSTIFICATION: findings outlive the borrowed source
                // inventory used to evaluate this workspace.
                file: file.clone(),
                line: u32::try_from(offset + 1).unwrap_or(u32::MAX),
                snippet: Some(String::from(line)),
            });
        }
    }
    findings
}

fn is_cargo_manifest(file: &RelPath) -> bool {
    file.as_str().ends_with("Cargo.toml")
}

fn package_name(source: &str) -> Option<&str> {
    let mut package_section = false;
    for line in source.lines() {
        if let Some(header) = section_header(line) {
            package_section = header == "package";
            continue;
        }
        if package_section {
            let (key, value) = line.split_once('=')?;
            if key.trim() == "name" {
                return value.trim().trim_matches('"').split_whitespace().next();
            }
        }
    }
    None
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

fn dependency_name(line: &str) -> Option<&str> {
    let (name, _) = line.split_once('=')?;
    Some(name.trim()).filter(|name| !name.is_empty() && !name.starts_with('#'))
}

#[cfg(test)]
mod tests {
    use super::findings_for_sources;
    use enforcer_domain::paths::RelPath;

    fn source(
        path: &str,
        contents: &str,
    ) -> Result<(RelPath, Option<String>), Box<dyn std::error::Error>> {
        Ok((path.parse::<RelPath>()?, Some(String::from(contents))))
    }

    #[test]
    fn allows_workspace_member_paths_and_rejects_arbitrary_local_paths(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let sources = vec![
            source(
                "Cargo.toml",
                "[workspace]\nmembers = [\"crates/core\", \"crates/app\"]\n\n[workspace.dependencies]\ncore = { path = \"crates/core\" }\n",
            )?,
            source("crates/core/Cargo.toml", "[package]\nname = \"core\"\n")?,
            source(
                "crates/app/Cargo.toml",
                "[package]\nname = \"app\"\n\n[dependencies]\ncore = { path = \"../core\" }\noutside = { path = \"../outside\" }\n",
            )?,
        ];
        let findings = findings_for_sources(&sources);
        let finding = findings
            .first()
            .ok_or_else(|| std::io::Error::other("expected one external path finding"))?;
        assert_eq!(finding.rule_id.as_str(), "RR-9.3");
        assert_eq!(finding.file.as_str(), "crates/app/Cargo.toml");
        assert_eq!(finding.line, 6);
        Ok(())
    }
}
