//! Mechanical retention proof for the complete CyberSkills corpus.
//!
//! This manifest deliberately distinguishes an explicitly evidenced mapping
//! from a related-looking native rule.  Until a row names both a target and
//! a per-skill evidence path, it remains unported or adapter-deferred.

use std::{collections::BTreeSet, path::PathBuf};

use enforcer_domain::rules_types::{RuleCatalogJson, RuleCatalogSource};
use enforcer_rules::loader::{load_registry_from_records, parse_catalog};
use serde_json::Value;

const DISPOSITION_JSON: &str = include_str!("../dispositions/cyberskills-disposition.json");
const NATIVE_RULES_JSON: &str = include_str!("../rules/cyberskills.json");
const ADAPTER_RULES_JSON: &str = include_str!("../rules/cyberskills-adapters.json");
const SOURCE_CATALOG: &str = include_str!(
    "../../../docs/plans/enforcer-selfhost-plan/refs/cyberskills-mechanization-catalog.md"
);

fn repo_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?)
}

fn registry_ids(json: &str, source: &str) -> Result<BTreeSet<String>, Box<dyn std::error::Error>> {
    let raw = RuleCatalogJson::try_from(json.to_owned())?;
    let source = RuleCatalogSource::try_from(source.to_owned())?;
    Ok(load_registry_from_records(parse_catalog(&raw, &source)?)?
        .iter()
        .map(|record| record.rule_id.as_str().to_owned())
        .collect())
}

fn field<'a>(record: &'a Value, name: &str) -> Result<&'a str, Box<dyn std::error::Error>> {
    record
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("record missing string {name}: {record}").into())
}

fn source_catalog_ids() -> BTreeSet<String> {
    let mut section = "";
    let mut ids = BTreeSet::new();
    for line in SOURCE_CATALOG.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            section = "";
            if let Some(name) = heading.split_whitespace().next() {
                section = name;
            }
            continue;
        }
        let table_id = line
            .strip_prefix("| ")
            .and_then(|row| row.split('|').next())
            .map(str::trim);
        if matches!(section, "T1" | "T2" | "ADAPTER") {
            if let Some(id) = table_id {
                if id != "skill" && id != "---" {
                    ids.insert(id.to_owned());
                }
            }
        }
        if section == "PROSE" {
            if let Some(id) = line.strip_prefix("- ") {
                ids.insert(id.trim().to_owned());
            }
        }
    }
    ids
}

#[test]
fn rule_catalog_directory_contains_only_valid_rule_record_arrays(
) -> Result<(), Box<dyn std::error::Error>> {
    let rules_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("rules");
    let mut catalog_paths: Vec<PathBuf> = std::fs::read_dir(&rules_dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("json"))
        .collect();
    catalog_paths.sort();
    assert!(
        !catalog_paths.is_empty(),
        "rule catalog directory must not be empty"
    );

    for path in catalog_paths {
        let raw = std::fs::read_to_string(&path)?;
        let source = path.display().to_string();
        let json = RuleCatalogJson::try_from(raw)?;
        let source = RuleCatalogSource::try_from(source)?;
        let records = parse_catalog(&json, &source)?;
        assert!(
            !records.is_empty(),
            "rule catalog must contain at least one record: {}",
            path.display()
        );
    }
    Ok(())
}

#[test]
fn disposition_covers_the_entire_catalog_once_with_honest_totals(
) -> Result<(), Box<dyn std::error::Error>> {
    let manifest: Value = serde_json::from_str(DISPOSITION_JSON)?;
    let records = manifest
        .get("records")
        .and_then(Value::as_array)
        .ok_or("manifest.records must be an array")?;
    let totals = manifest.get("totals").ok_or("manifest.totals missing")?;

    assert_eq!(
        records.len(),
        817,
        "every catalog skill needs one disposition"
    );
    assert_eq!(totals["catalogRows"], 817);
    assert_eq!(totals["nativeMapped"], 0);
    assert_eq!(totals["unported"], 282);
    assert_eq!(totals["adapterDeferred"], 399);
    assert_eq!(totals["advisoryProse"], 136);

    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let mut disposition_counts = std::collections::BTreeMap::new();
    for record in records {
        let id = field(record, "catalogId")?;
        let path = field(record, "sourcePath")?;
        assert!(ids.insert(id.to_owned()), "duplicate catalog id: {id}");
        assert!(
            paths.insert(path.to_owned()),
            "duplicate source path: {path}"
        );
        assert_eq!(
            path,
            format!("vendor/anthropic-cybersecurity-skills/skills/{id}/SKILL.md"),
            "catalog path must remain canonical"
        );
        let disposition = field(record, "disposition")?;
        assert!(
            matches!(
                disposition,
                "native" | "unported" | "adapter-deferred" | "advisory-prose"
            ),
            "unknown disposition: {disposition}"
        );
        *disposition_counts.entry(disposition).or_insert(0usize) += 1;
        assert!(!field(record, "rationale")?.trim().is_empty());
    }
    assert_eq!(disposition_counts.get("unported"), Some(&282));
    assert_eq!(disposition_counts.get("adapter-deferred"), Some(&399));
    assert_eq!(disposition_counts.get("advisory-prose"), Some(&136));
    assert_eq!(disposition_counts.get("native"), None);
    assert_eq!(
        ids,
        source_catalog_ids(),
        "manifest ids must exactly match the checked-in source catalog"
    );
    Ok(())
}

#[test]
fn explicit_registry_targets_exist_and_have_per_skill_evidence(
) -> Result<(), Box<dyn std::error::Error>> {
    let manifest: Value = serde_json::from_str(DISPOSITION_JSON)?;
    let records = manifest["records"]
        .as_array()
        .ok_or("manifest.records missing")?;
    let root = repo_root()?;
    let native_ids = registry_ids(NATIVE_RULES_JSON, "rules/cyberskills.json")?;
    let adapter_ids = registry_ids(ADAPTER_RULES_JSON, "rules/cyberskills-adapters.json")?;
    let mut mapped = 0usize;

    for record in records {
        let id = field(record, "catalogId")?;
        let disposition = field(record, "disposition")?;
        let native = record.get("nativeRuleId").and_then(Value::as_str);
        let adapter = record.get("adapterRuleId").and_then(Value::as_str);
        match disposition {
            "native" => {
                let rule_id =
                    native.ok_or_else(|| format!("native row {id} has no nativeRuleId"))?;
                assert!(
                    adapter.is_none(),
                    "native row {id} cannot also name adapterRuleId"
                );
                assert!(
                    native_ids.contains(rule_id),
                    "native target missing from registry: {rule_id}"
                );
                mapped += 1;
            }
            "adapter-deferred" if adapter.is_some() => {
                let rule_id =
                    adapter.ok_or_else(|| format!("adapter row {id} has no adapterRuleId"))?;
                assert!(
                    native.is_none(),
                    "adapter row {id} cannot also name nativeRuleId"
                );
                assert!(
                    adapter_ids.contains(rule_id),
                    "adapter target missing from registry: {rule_id}"
                );
                mapped += 1;
            }
            "adapter-deferred" | "unported" | "advisory-prose" => {
                assert!(
                    native.is_none() && adapter.is_none(),
                    "unmapped row {id} names a registry target"
                );
            }
            other => return Err(format!("unsupported disposition for {id}: {other}").into()),
        }
        if native.is_some() || adapter.is_some() {
            let evidence = field(record, "evidencePath")?;
            assert!(
                root.join(evidence).is_file(),
                "mapping evidence missing for {id}: {evidence}"
            );
        }
    }

    assert_eq!(
        mapped, 0,
        "there is currently no explicit per-skill vendor-to-registry mapping; do not infer one from theme similarity"
    );
    Ok(())
}
