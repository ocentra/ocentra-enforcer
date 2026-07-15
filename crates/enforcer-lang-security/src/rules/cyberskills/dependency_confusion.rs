//! `CYBER-DEPCONFUSION.1` (T2 heuristic advisory) — harvest target 7 (h11
//! workpack): manifest name parsing derived from
//! `vendor/anthropic-cybersecurity-skills/skills/detecting-dependency-confusion/scripts/agent.py`.
//!
//! # Honest divergence from the vendor (NOT a 1:1 port)
//!
//! The vendor's actual claimability verdict is a NETWORK check: for each
//! manifest name it performs an HTTP request to the public registry
//! (`http_status`, agent.py L140) and reports `CLAIMABLE` only on a 404
//! (`status != 200`, L146). Its `is_secure(name, patterns)` helper (L95-96)
//! is NOT an "internal name" detector — it is a user-supplied
//! `--secure-namespaces` glob allowlist used to SKIP names before the
//! lookup (empty by default). So there is no offline predicate in the
//! vendor that decides claimability; the deciding signal is the live
//! registry probe.
//!
//! That network probe cannot run in h11 (native-only, no subprocess/HTTP —
//! registry-verified detection is the `h12` adapter pack's job). This
//! validator is therefore a deterministic NAMING-CONVENTION HEURISTIC, not
//! a faithful reproduction of the vendor verdict: it flags an UNSCOPED
//! dependency whose name matches a common organization-private convention
//! (`internal-*` / `corp-*` / `private-*`) as a CANDIDATE for dependency
//! confusion — "this looks like an internal package published unscoped;
//! confirm it is claimed/reserved on the public registry". Scoped
//! (`@org/name`) packages are treated as not-claimable (npm scoping is the
//! standard mitigation). It never asserts a proven takeover, and it will
//! neither find every claimable name the registry probe would (no network)
//! nor apply the vendor's `--secure-namespaces` skip list.

use enforcer_domain::boundary::decode_error::DecodeError;
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

/// Common organization-private naming prefixes. An UNSCOPED dependency
/// whose name starts with one of these signals "meant to be internal-only"
/// and, published unscoped, is a candidate for dependency-confusion
/// takeover. This is a heuristic convention (see the module docs), not a
/// vendor-defined list — the vendor decides claimability by a live
/// registry probe, which h11 cannot perform.
const INTERNAL_PREFIXES: &[&str] = &["internal-", "corp-", "private-"];

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

/// Return the package actually resolved by an npm alias specifier.
///
/// A dependency such as `"public-name": "npm:internal-api@^1"` downloads
/// `internal-api`, not `public-name`. Inspecting only manifest keys would
/// therefore miss an unscoped internal-looking package hidden behind an
/// alias. Scoped alias targets retain their complete `@scope/name` form so
/// the normal scope mitigation is applied by [`looks_internal`].
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
            for (name, specifier) in map {
                names.push(name);
                if let Some(target) = specifier.strip_prefix("npm:") {
                    // An npm alias can hide the actually resolved package:
                    // `public-name: npm:internal-api@^1` fetches
                    // `internal-api`. A scoped target starts with `@`; only
                    // a later `@` is its version delimiter.
                    let target = if target.starts_with('@') {
                        target
                            .rsplit_once('@')
                            .and_then(|(name, _)| (!name.is_empty()).then_some(name))
                            .unwrap_or(target)
                    } else {
                        target.split_once('@').map_or(target, |(name, _)| name)
                    };
                    if !target.is_empty() {
                        names.push(target);
                    }
                }
            }
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
                title: "dependency name is a dependency-confusion candidate (heuristic)".to_owned(),
                detail: format!(
                    "dependency `{name}` is unscoped and matches an internal-looking naming \
                     convention, so it is a CANDIDATE for a dependency-confusion takeover. This \
                     is a naming heuristic, not a registry-verified verdict (see h12 for the \
                     registry-probe adapter). Fix: publish it under an org scope \
                     (`@your-org/{name}`), or confirm the public-registry name is \
                     claimed/reserved by your org."
                ),
                file: input.file.clone(),
                line: 1,
                snippet: None,
            });
        }
        findings
    }
}
