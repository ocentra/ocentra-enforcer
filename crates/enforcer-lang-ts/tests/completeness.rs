//! Count-parity completeness test (workpack requirement): every
//! `rules/rules.json` entry with `language == "typescript"` must have a
//! registered [`enforcer_lang_ts::rules::registry`] row (no orphan rule
//! id), the registry must carry no id NOT present in `rules.json`, there
//! must be no duplicate rule id in the registry, and the total must equal
//! 73 — the authoritative TS rule count this workpack cites. The test
//! FAILS if `rules/rules.json` gains/loses a typescript rule without a
//! matching validator + fixtures landing in this crate.

use std::collections::BTreeSet;

use enforcer_lang_ts::rules::registry::build_all;

/// The authoritative rule catalog, embedded at compile time from the
/// repo-root `rules/rules.json` (the same file the workpack's rule
/// inventory table cites). `CARGO_MANIFEST_DIR` is
/// `<repo>/crates/enforcer-lang-ts`, so the catalog is two levels up.
const RULES_JSON: &str = include_str!("../../../rules/rules.json");

fn typescript_rule_ids_from_catalog() -> Result<BTreeSet<String>, Box<dyn std::error::Error>> {
    let catalog: serde_json::Value = serde_json::from_str(RULES_JSON)?;
    let rules = catalog
        .get("rules")
        .and_then(serde_json::Value::as_array)
        .ok_or("rules/rules.json: missing top-level `rules` array")?;
    let mut ids = BTreeSet::new();
    for rule in rules {
        let language = rule.get("language").and_then(serde_json::Value::as_str);
        if language != Some("typescript") {
            continue;
        }
        let id = rule
            .get("id")
            .and_then(serde_json::Value::as_str)
            .ok_or("rules/rules.json: typescript rule missing `id`")?;
        ids.insert(id.to_owned());
    }
    Ok(ids)
}

#[test]
fn registry_covers_every_typescript_rule_id_with_no_orphans_and_no_duplicates(
) -> Result<(), Box<dyn std::error::Error>> {
    let catalog_ids = typescript_rule_ids_from_catalog()?;
    assert_eq!(
        catalog_ids.len(),
        73,
        "rules/rules.json typescript count drifted from the workpack's authoritative 73"
    );

    let rows = build_all()?;

    let mut seen = BTreeSet::new();
    for row in &rows {
        assert!(
            seen.insert(row.rule_id.to_owned()),
            "duplicate registry row for rule id `{}`",
            row.rule_id
        );
    }

    let registry_ids: BTreeSet<String> = rows.iter().map(|row| row.rule_id.to_owned()).collect();

    let orphan_catalog_ids: Vec<_> = catalog_ids.difference(&registry_ids).collect();
    assert!(
        orphan_catalog_ids.is_empty(),
        "rules/rules.json typescript rule id(s) with no registered validator: {orphan_catalog_ids:?}"
    );

    let unknown_registry_ids: Vec<_> = registry_ids.difference(&catalog_ids).collect();
    assert!(
        unknown_registry_ids.is_empty(),
        "registry rule id(s) not present in rules/rules.json's typescript set: {unknown_registry_ids:?}"
    );

    assert_eq!(
        registry_ids.len(),
        73,
        "registry total must equal the workpack's authoritative typescript count (73)"
    );

    Ok(())
}
