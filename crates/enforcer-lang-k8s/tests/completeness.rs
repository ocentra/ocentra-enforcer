//! Canonical Kubernetes-rule-to-registry completeness proof.

use std::collections::BTreeSet;

use enforcer_domain::ids::{BuiltInK8sRule, RuleId};
use enforcer_domain::paths::{RelPath, RepoRoot};
use enforcer_lang_k8s::rules::registry::build_all;
use enforcer_validator::harness::run_fixture_parity;

#[test]
fn registry_covers_every_canonical_k8s_rule_without_duplicates(
) -> Result<(), Box<dyn std::error::Error>> {
    let canonical_ids: BTreeSet<RuleId> = BuiltInK8sRule::ALL
        .into_iter()
        .map(BuiltInK8sRule::id)
        .collect();
    assert_eq!(canonical_ids.len(), BuiltInK8sRule::ALL.len());

    let rows = build_all()?;
    let registry_ids: BTreeSet<&RuleId> = rows.iter().map(|row| row.validator.rule_id()).collect();
    assert_eq!(
        rows.len(),
        registry_ids.len(),
        "registry ids must be unique"
    );
    for canonical_id in &canonical_ids {
        assert!(
            registry_ids
                .iter()
                .any(|registered_id| *registered_id == canonical_id),
            "canonical rule `{canonical_id}` is missing from the registry"
        );
    }
    Ok(())
}

#[test]
fn every_canonical_k8s_rule_passes_fixture_parity() -> Result<(), Box<dyn std::error::Error>> {
    let root = RepoRoot::try_from(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))?;
    let rows = build_all()?;
    assert_eq!(rows.len(), BuiltInK8sRule::ALL.len());

    for (row, rule) in rows.iter().zip(BuiltInK8sRule::ALL) {
        assert_eq!(row.validator.rule_id(), &rule.id());
        let slug = rule.id().as_str().to_ascii_lowercase().replace('.', "-");
        let fail: RelPath = format!("fixtures/generic-scanner/{slug}/fail.yaml").parse()?;
        let pass: RelPath = format!("fixtures/generic-scanner/{slug}/pass.yaml").parse()?;
        run_fixture_parity(row.validator.as_ref(), &root, &fail, &pass)?;
    }
    Ok(())
}
