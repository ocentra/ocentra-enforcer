//! Catalog-to-registry completeness proof for built-in IaC rules.

use std::collections::BTreeSet;

use enforcer_domain::ids::{BuiltInIacRule, RuleId};
use enforcer_lang_iac::rules::registry::build_all;

const RULES_JSON: &str = include_str!("../../../rules/rules.json");

fn iac_rule_ids_from_catalog() -> Result<BTreeSet<RuleId>, Box<dyn std::error::Error>> {
    let catalog: serde_json::Value = serde_json::from_str(RULES_JSON)?;
    let Some(rules) = catalog.get("rules").and_then(serde_json::Value::as_array) else {
        return Err(std::io::Error::other("catalog lacks a rules array").into());
    };
    let mut ids = BTreeSet::new();
    for rule in rules {
        if rule.get("language").and_then(serde_json::Value::as_str) != Some("iac") {
            continue;
        }
        let Some(id) = rule.get("id").and_then(serde_json::Value::as_str) else {
            return Err(std::io::Error::other("IaC catalog row lacks an id").into());
        };
        ids.insert(id.parse()?);
    }
    Ok(ids)
}

#[test]
fn registry_covers_every_canonical_iac_rule_without_duplicates(
) -> Result<(), Box<dyn std::error::Error>> {
    let catalog_ids = iac_rule_ids_from_catalog()?;
    let canonical_ids: BTreeSet<RuleId> = BuiltInIacRule::ALL
        .into_iter()
        .map(BuiltInIacRule::id)
        .collect();
    assert_eq!(catalog_ids, canonical_ids);

    let rows = build_all()?;
    let registry_ids: BTreeSet<RuleId> =
        rows.iter().map(|row| Clone::clone(&row.rule_id)).collect();
    assert_eq!(
        rows.len(),
        registry_ids.len(),
        "registry ids must be unique"
    );
    assert_eq!(registry_ids, canonical_ids);
    Ok(())
}
