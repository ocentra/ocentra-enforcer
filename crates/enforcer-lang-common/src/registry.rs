//! Aggregates every common-family [`Validator`](enforcer_validator::validator::Validator)
//! into one list: the 29 `PatternValidator`-backed family modules under
//! [`crate::families`] plus the bespoke `PORT-1.1`
//! [`crate::port_platform::PortabilityValidator`]. `all()` is this crate's
//! single entry point for "every validator I own"; the count-parity test
//! (`tests/parity.rs`) asserts its length plus rule-id set against
//! `rules/rules.json` minus the SEC-2 family delegated to arc-10.
//!
//! `families::arch_1` contributes 16 rows (not 15) as of `ARCH-1.16`
//! (`Presentation/UI cannot call business logic directly`), bringing the
//! family-module total to 249 and the grand total (with `PORT-1.1`) to 250.

use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::families;
use crate::pattern::PatternValidator;
use crate::port_platform::{DeclaredScope, PortabilityValidator};

/// Push one [`PatternValidator`] into `out`, decoding `rule_id` at the call
/// site. Family modules call this once per row in their static table.
/// Silently skips (rather than panics) a malformed literal — the
/// count-parity test in `tests/parity.rs` catches the resulting gap by
/// comparing against `rules.json`, keeping this crate `unwrap`/`expect`-free
/// per workspace lint policy while still failing the build loudly on drift.
pub fn reg(
    out: &mut Vec<Box<dyn Validator>>,
    rule_id: &str,
    title: &'static str,
    severity: Severity,
    marker: &'static str,
) {
    if let Ok(id) = rule_id.parse::<RuleId>() {
        out.push(Box::new(PatternValidator::new(
            id,
            title,
            severity,
            [marker],
        )));
    }
}

/// Every validator this crate owns: all 29 family modules concatenated,
/// plus the bespoke `PORT-1.1` platform-scoped validator. `port_scope` is
/// threaded through from the caller's own knowledge of whether the
/// project's config source actually declared `supportedPlatforms` — pass
/// [`DeclaredScope::Undeclared`] when it did not, per PORT-1.1's
/// "no silent relaxation by omission" requirement (see
/// `crate::port_platform` for why this can't just be
/// `EffectiveConfig::supported_platforms` verbatim).
pub fn all(port_scope: DeclaredScope) -> Vec<Box<dyn Validator>> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    v.extend(families::ai_1::validators());
    v.extend(families::arch_1::validators());
    v.extend(families::bound_1::validators());
    v.extend(families::cfg_1::validators());
    v.extend(families::ci_1::validators());
    v.extend(families::contract_1::validators());
    v.extend(families::dep_1::validators());
    v.extend(families::doc_1::validators());
    v.extend(families::docenf_1::validators());
    v.extend(families::enf_1::validators());
    v.extend(families::enf_2::validators());
    v.extend(families::gen_1::validators());
    v.extend(families::gen_2::validators());
    v.extend(families::har_1::validators());
    v.extend(families::har_2::validators());
    v.extend(families::lit_1::validators());
    v.extend(families::mcp_1::validators());
    v.extend(families::npm_1::validators());
    v.extend(families::proof_1::validators());
    v.extend(families::repo_1::validators());
    v.extend(families::sbom_1::validators());
    v.extend(families::scan_1::validators());
    v.extend(families::scan_2::validators());
    v.extend(families::sec_1::validators());
    v.extend(families::src_1::validators());
    v.extend(families::src_2::validators());
    v.extend(families::test_1::validators());
    v.extend(families::test_2::validators());
    v.extend(families::waiver_1::validators());
    if let Ok(port_id) = "PORT-1.1".parse::<RuleId>() {
        v.push(Box::new(PortabilityValidator::new(port_id, port_scope)));
    }
    v
}

#[cfg(test)]
mod tests {
    use super::all;
    use crate::port_platform::DeclaredScope;

    #[test]
    fn all_returns_the_expected_total_validator_count() {
        // 249 PatternValidator rows (29 families, incl. ARCH-1.16) + 1
        // PORT-1.1 = 250, the count-parity target (270 language==common
        // rules minus the 20 SEC-2 rows delegated to arc-10 per the
        // workpack's SEC-2 decision).
        assert_eq!(all(DeclaredScope::Undeclared).len(), 250);
    }

    #[test]
    fn every_registered_validator_has_a_unique_rule_id() {
        use std::collections::BTreeSet;
        let validators = all(DeclaredScope::Undeclared);
        let mut seen = BTreeSet::new();
        for validator in &validators {
            let id = validator.rule_id().to_string();
            assert!(seen.insert(id.clone()), "duplicate ruleId `{id}`");
        }
        assert_eq!(seen.len(), 250);
    }
}
