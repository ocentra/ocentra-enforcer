//! Count-parity completeness test: every rule id in
//! [`enforcer_lang_k8s::rules::spec::SPECS`] must have exactly one
//! registered [`enforcer_lang_k8s::rules::registry`] row (no orphan spec,
//! no duplicate registry row), and the registry total must equal
//! `SPECS.len()` (10, the arc-12 K8S-family rule count). This is the same
//! shape the sibling lang crates' completeness tests take against
//! `rules/rules.json`, scoped here to this crate's own spec table since
//! `rules/rules.json` carries no `language == "k8s"` rows yet.

use std::collections::BTreeSet;

use enforcer_domain::ids::RuleId;
use enforcer_lang_k8s::rules::registry::build_all;
use enforcer_lang_k8s::rules::spec::SPECS;

#[test]
fn registry_covers_every_k8s_spec_with_no_orphans_and_no_duplicates(
) -> Result<(), Box<dyn std::error::Error>> {
    let spec_ids: BTreeSet<RuleId> = SPECS
        .iter()
        .map(|spec| spec.rule_id())
        .collect::<Result<_, _>>()?;
    assert_eq!(
        spec_ids.len(),
        10,
        "K8S-family spec count drifted from the workpack's authoritative 10"
    );

    let rows = build_all()?;

    let mut seen = BTreeSet::new();
    for row in &rows {
        assert!(
            seen.insert(row.rule_id.clone()),
            "duplicate registry row for rule id `{}`",
            row.rule_id
        );
    }

    let registry_ids: BTreeSet<RuleId> = rows.iter().map(|row| row.rule_id.clone()).collect();

    let orphan_spec_ids: Vec<_> = spec_ids.difference(&registry_ids).collect();
    assert!(
        orphan_spec_ids.is_empty(),
        "spec rule id(s) with no registered validator: {orphan_spec_ids:?}"
    );

    let unknown_registry_ids: Vec<_> = registry_ids.difference(&spec_ids).collect();
    assert!(
        unknown_registry_ids.is_empty(),
        "registry rule id(s) not present in the spec table: {unknown_registry_ids:?}"
    );

    assert_eq!(
        registry_ids.len(),
        10,
        "registry total must equal the workpack's authoritative K8S count (10)"
    );

    Ok(())
}
