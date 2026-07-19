//! Package-manifest decoding boundary.
//! Malformed JSON is rejected, with negative coverage in this module's tests.

use std::collections::BTreeMap;

#[derive(Debug, Default, serde::Deserialize)]
struct PackageManifest {
    // DEFAULT-JUSTIFICATION: a missing dependency section contributes no package names.
    #[serde(default)]
    dependencies: BTreeMap<String, String>,
    // DEFAULT-JUSTIFICATION: a missing development dependency section contributes no names.
    #[serde(default, rename = "devDependencies")]
    dev_dependencies: BTreeMap<String, String>,
    // DEFAULT-JUSTIFICATION: a missing optional dependency section contributes no names.
    #[serde(default, rename = "optionalDependencies")]
    optional_dependencies: BTreeMap<String, String>,
    // DEFAULT-JUSTIFICATION: a missing peer dependency section contributes no names.
    #[serde(default, rename = "peerDependencies")]
    peer_dependencies: BTreeMap<String, String>,
}

pub(crate) fn resolved_names(source: &str) -> Option<Vec<String>> {
    let manifest: PackageManifest = serde_json::from_str(source).ok()?;
    let mut names = Vec::new();
    for map in [
        manifest.dependencies,
        manifest.dev_dependencies,
        manifest.optional_dependencies,
        manifest.peer_dependencies,
    ] {
        for (name, specifier) in map {
            names.push(name);
            if let Some(target) = specifier.strip_prefix("npm:") {
                let target = if target.starts_with('@') {
                    target
                        .rsplit_once('@')
                        .and_then(|(name, _)| (!name.is_empty()).then_some(name))
                        .unwrap_or(target)
                } else {
                    target.split_once('@').map_or(target, |(name, _)| name)
                };
                if !target.is_empty() {
                    names.push(target.to_owned());
                }
            }
        }
    }
    names.sort_unstable();
    names.dedup();
    Some(names)
}

pub(crate) fn looks_internal(name: &str) -> bool {
    const INTERNAL_PREFIXES: &[&str] = &["internal-", "corp-", "private-"];
    !name.starts_with('@')
        && INTERNAL_PREFIXES
            .iter()
            .any(|prefix| name.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    #[test]
    fn malformed_package_manifest_is_rejected() {
        assert!(super::resolved_names(r#"{"dependencies":"#).is_none());
    }
}
