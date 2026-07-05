//! Map a parsed [`super::PolicySpec`] against the [`super::BackedRuleCatalog`]
//! to produce a [`super::MechanizedProfile`] plus unbacked-rule
//! [`Finding`]s (h08, POLICY-SPEC-INGESTION — the honesty-seam stage).
//!
//! This is the point of the whole module: a rule the spec ASSERTS but that
//! has no real [`enforcer_validator::validator::Validator`] behind it must
//! never become an ENABLED profile row masquerading as enforced. Backed
//! rules become `backed: true` rows (actually enforced, T1
//! block/T2 score/T3 label per the spec's own tier). Unbacked rules still
//! appear in the profile (so the UI/d01 pipeline can see what was asserted)
//! but carry `backed: false`, and this function additionally emits one
//! structured [`Finding`] per unbacked rule, flagging it for mechanization
//! — never a silent accept-as-enforced.

use enforcer_domain::findings::Finding;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;

use super::backing::BackedRuleCatalog;
use super::spec::{MechanizedProfile, PolicySpec, ProfileRuleRow};

/// Map `spec` (already parsed) against `catalog` (the backed-rule
/// snapshot) into a neutral [`MechanizedProfile`] named `profile_name`,
/// plus a `Finding` for every rule the spec asserts that `catalog` does
/// not back — the un-mechanized subset, fed to d01/d08 for scaffolding
/// rather than silently enabled.
///
/// `file` is the `RelPath` the emitted findings point at (the ingested
/// spec's own path, matching how every other Track H validator attributes
/// a finding to the source it fired on).
pub fn map_to_profile(
    profile_name: &str,
    spec: &PolicySpec,
    catalog: &BackedRuleCatalog,
    file: &RelPath,
) -> (MechanizedProfile, Vec<Finding>) {
    let mut rows = Vec::with_capacity(spec.asserted_rules.len());
    let mut findings = Vec::new();

    for asserted in &spec.asserted_rules {
        let backed = catalog.is_backed(asserted.rule_id.as_str());
        rows.push(ProfileRuleRow {
            rule_id: asserted.rule_id.clone(),
            tier: asserted.tier,
            backed,
        });

        if backed {
            continue;
        }

        findings.push(Finding {
            rule_id: asserted.rule_id.clone(),
            severity: Severity::Warning,
            title: "policy spec asserts a rule with no mechanized backing (flagged, not enabled)"
                .to_owned(),
            detail: format!(
                "the ingested policy spec asserts rule `{}` at tier {:?}, but no mechanized \
                 `Validator` backs that rule id yet. Per the ingestion honesty seam, an \
                 asserted-but-unbacked rule is NEVER silently treated as enforced — it is \
                 flagged here for mechanization (feed to d01's rule-scaffold engine / d08) so a \
                 real `Validator` can be built for it. Fix: scaffold the rule via `enforcer rule \
                 new {}` (d01) and register it in the crate's Validator seam, or remove the \
                 assertion from the spec if it is intentionally out of mechanized scope.",
                asserted.rule_id.as_str(),
                asserted.tier,
                asserted.rule_id.as_str(),
            ),
            file: file.clone(),
            line: asserted.line,
            snippet: None,
        });
    }

    let profile = MechanizedProfile {
        profile_name: profile_name.to_owned(),
        required_test_categories: spec.required_test_categories.clone(),
        invariants: spec.invariants.clone(),
        rules: rows.into_iter().map(Into::into).collect(),
    };

    (profile, findings)
}
