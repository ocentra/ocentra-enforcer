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

/// Common organization-private naming prefixes. An UNSCOPED dependency
/// whose name starts with one of these signals "meant to be internal-only"
/// and, published unscoped, is a candidate for dependency-confusion
/// takeover. This is a heuristic convention (see the module docs), not a
/// vendor-defined list — the vendor decides claimability by a live
/// registry probe, which h11 cannot perform.
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
#[derive(Debug)]
pub struct DependencyConfusionClaimableValidator {
    rule_id: RuleId,
}

impl DependencyConfusionClaimableValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: enforcer_domain::ids::BuiltInSecurityRule::CyberDependencyConfusion.id(),
        })
    }
}

impl Validator for DependencyConfusionClaimableValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Some(names) =
            crate::boundary::dependency_manifest::resolved_names(input.source.as_str())
        else {
            return Vec::new();
        };
        let mut findings = Vec::new();
        for name in names {
            if !crate::boundary::dependency_manifest::looks_internal(&name) {
                continue;
            }
            findings.extend(crate::boundary::finding::from_owned_source(
                (&self.rule_id, Severity::Warning),
                "dependency name is a dependency-confusion candidate (heuristic)",
                format!(
                    "dependency `{name}` is unscoped and matches an internal-looking naming \
                     convention, so it is a CANDIDATE for a dependency-confusion takeover. This \
                     is a naming heuristic, not a registry-verified verdict (see h12 for the \
                     registry-probe adapter). Fix: publish it under an org scope \
                     (`@your-org/{name}`), or confirm the public-registry name is \
                     claimed/reserved by your org."
                ),
                input.file,
                (1, None),
            ));
        }
        findings
    }
}
