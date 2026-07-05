//! `CYBER-DEPCONFUSION.1` (T1) — harvest target 7 (h11 workpack): manifest
//! name parsing ported from
//! `vendor/anthropic-cybersecurity-skills/skills/detecting-dependency-confusion/scripts/agent.py`
//! (L46-92)'s `parse_npm`/`is_secure` logic. The original agent.py resolves
//! each dependency name against the public npm registry (an HTTP call) to
//! see whether an internal-looking package name is claimable by an
//! attacker; this validator drops the network lookup and instead applies
//! the SAME "internal-looking name" predicate the corpus script's
//! `is_secure` helper checks BEFORE it ever calls the registry: an
//! unscoped dependency name that looks internal (matches an
//! organization-private naming convention: a bare `internal-*`/`corp-*`/
//! `private-*` prefix, or a name with no `@scope/` and no registry
//! pin/lockfile-resolved integrity hash) is flagged as
//! dependency-confusion-CLAIMABLE — never as a proven live takeover
//! (that verdict needs the registry-404 check this crate does not
//! perform).

use enforcer_core::error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

#[derive(Debug, Default, serde::Deserialize)]
struct PackageManifest {
    #[serde(default)]
    dependencies: std::collections::BTreeMap<String, String>,
    #[serde(default, rename = "devDependencies")]
    dev_dependencies: std::collections::BTreeMap<String, String>,
    #[serde(default, rename = "optionalDependencies")]
    optional_dependencies: std::collections::BTreeMap<String, String>,
    #[serde(default, rename = "peerDependencies")]
    peer_dependencies: std::collections::BTreeMap<String, String>,
}

/// Internal-looking naming prefixes, mirroring the org-private naming
/// convention `is_secure` in the corpus script guards against (an unscoped
/// package whose name signals "this is meant to be internal-only" is
/// claimable on the public registry unless it is scoped).
const INTERNAL_PREFIXES: &[&str] = &["internal-", "corp-", "private-", "acme-"];

fn looks_internal(name: &str) -> bool {
    if name.starts_with('@') {
        // Scoped packages (`@org/name`) are not claimable by an unrelated
        // publisher on the public registry the same way an unscoped name
        // is — scoping is npm's own dependency-confusion mitigation.
        return false;
    }
    INTERNAL_PREFIXES
        .iter()
        .any(|prefix| name.starts_with(prefix))
}

/// `CYBER-DEPCONFUSION.1` — an unscoped, internal-looking dependency name
/// in a `package.json` manifest is flagged as dependency-confusion
/// claimable.
pub struct DependencyConfusionClaimableValidator {
    rule_id: RuleId,
}

impl DependencyConfusionClaimableValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CYBER-DEPCONFUSION.1".parse()?,
        })
    }
}

impl Validator for DependencyConfusionClaimableValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Ok(manifest) = serde_json::from_str::<PackageManifest>(input.source) else {
            return Vec::new();
        };
        let mut names: Vec<&str> = Vec::new();
        for map in [
            &manifest.dependencies,
            &manifest.dev_dependencies,
            &manifest.optional_dependencies,
            &manifest.peer_dependencies,
        ] {
            names.extend(map.keys().map(String::as_str));
        }
        names.sort_unstable();
        names.dedup();

        let mut findings = Vec::new();
        for name in names {
            if !looks_internal(name) {
                continue;
            }
            findings.push(Finding {
                rule_id: self.rule_id.clone(),
                severity: Severity::Warning,
                title: "dependency name is dependency-confusion claimable".to_owned(),
                detail: format!(
                    "dependency `{name}` is unscoped and matches an internal-looking naming \
                     convention. Fix: publish it under an org scope (`@your-org/{name}`), or \
                     confirm the public-registry name is claimed/reserved by your org to \
                     prevent a dependency-confusion takeover."
                ),
                file: input.file.clone(),
                line: 1,
                snippet: None,
            });
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;

    use super::DependencyConfusionClaimableValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn cyberskills_dependency_confusion() -> Result<(), Box<dyn std::error::Error>> {
        let validator = DependencyConfusionClaimableValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/cyberskills/supplychain.dependency-confusion-claimable/bad/package.json",
            "tests/fixtures/cyberskills/supplychain.dependency-confusion-claimable/good/package.json",
        )?;
        Ok(())
    }
}
