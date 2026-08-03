//! CP00 truth-ledger tests for CyberSkills identity and decomposition.

use std::{collections::BTreeSet, path::PathBuf};

use enforcer_domain::rules_types::{RuleCatalogJson, RuleCatalogSource};
use enforcer_rules::{
    cyberskills_disposition::{
        parse_manifest, validate_manifest, ComponentKind, ComponentStatus, DecompositionState,
        SourceAvailability, PROTECTED_CATALOG_ID, PROTECTED_SOURCE_PATH, PROTECTED_TRACKED_BLOB,
    },
    loader::{load_registry_from_records, parse_catalog},
};
use proptest::prelude::any;
use proptest::proptest;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const DISPOSITION_JSON: &str = include_str!("../dispositions/cyberskills-disposition.json");
const NATIVE_RULES_JSON: &str = include_str!("../rules/cyberskills.json");
const ADAPTER_RULES_JSON: &str = include_str!("../rules/cyberskills-adapters.json");
const SOURCE_CATALOG: &str = include_str!(
    "../../../docs/plans/enforcer-selfhost-plan/refs/cyberskills-mechanization-catalog.md"
);
const NEGATIVE_FIXTURES: &str =
    include_str!("fixtures/cyberskills_disposition/negative_cases.json");

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

fn source_catalog_ids() -> BTreeSet<String> {
    let mut section = "";
    let mut ids = BTreeSet::new();
    for line in SOURCE_CATALOG.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            section = heading.split_whitespace().next().unwrap_or("");
            continue;
        }
        if matches!(section, "T1" | "T2" | "ADAPTER") {
            if let Some(id) = line
                .strip_prefix("| ")
                .and_then(|row| row.split('|').next())
                .map(str::trim)
                .filter(|id| *id != "skill" && *id != "---" && !id.is_empty())
            {
                ids.insert(id.to_owned());
            }
        } else if section == "PROSE" {
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
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&rules_dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect();
    paths.sort();
    assert_eq!(paths.len(), 36);
    for path in paths {
        let raw = std::fs::read_to_string(&path)?;
        let json = RuleCatalogJson::try_from(raw)?;
        let source = RuleCatalogSource::try_from(path.display().to_string())?;
        let _records = parse_catalog(&json, &source)?;
    }
    Ok(())
}

#[test]
fn manifest_preserves_identities_and_derives_counts() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = parse_manifest(DISPOSITION_JSON)?;
    let counts = validate_manifest(&manifest)?;
    assert_eq!(counts.identity_rows, 817);
    assert_eq!(counts.readable_sources, 816);
    assert_eq!(counts.source_unavailable, 1);
    assert_eq!(counts.reviewed_rows, 6);
    assert_eq!(counts.decomposed_rows, 6);
    assert_eq!(counts.implemented_components, 6);
    assert_eq!(counts.proved_components, 0);
    assert_eq!(counts.advisory_retained, 0);
    assert_eq!(counts.manual_retained, 0);
    assert_eq!(counts.unexplained_rows, 810);
    assert_eq!(source_catalog_ids().len(), 817);
    assert_eq!(
        manifest
            .records
            .iter()
            .map(|record| record.catalog_id.as_str())
            .collect::<BTreeSet<_>>(),
        source_catalog_ids().iter().map(String::as_str).collect()
    );
    let root = repo_root()?;
    for record in manifest
        .records
        .iter()
        .filter(|record| record.source_availability == SourceAvailability::Available)
    {
        let source = std::fs::read(root.join(&record.source_path))?;
        assert_eq!(
            format!("{:x}", Sha256::digest(source)),
            record
                .source_sha256
                .as_deref()
                .ok_or("available record missing source hash")?,
            "available source fingerprint drifted for {}",
            record.catalog_id
        );
    }

    let unavailable = manifest
        .records
        .iter()
        .find(|record| record.catalog_id == PROTECTED_CATALOG_ID)
        .ok_or("protected identity must remain in the ledger")?;
    assert_eq!(unavailable.source_path, PROTECTED_SOURCE_PATH);
    assert_eq!(
        unavailable.source_availability,
        SourceAvailability::SourceUnavailable
    );
    assert_eq!(
        unavailable.decomposition_state,
        DecompositionState::Unavailable
    );
    assert!(unavailable.source_sha256.is_none());
    assert_eq!(unavailable.source_anchors.len(), 0);
    assert_eq!(unavailable.components.len(), 0);
    assert_eq!(
        unavailable
            .unavailable_source
            .as_ref()
            .ok_or("protected identity missing unavailableSource")?["trackedBlob"],
        PROTECTED_TRACKED_BLOB
    );
    Ok(())
}

#[test]
fn manifest_round_trips_without_count_drift() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = parse_manifest(DISPOSITION_JSON)?;
    let before = validate_manifest(&manifest)?;
    let encoded = serde_json::to_string(&manifest)?;
    let reparsed = parse_manifest(&encoded)?;
    let after = validate_manifest(&reparsed)?;
    assert_eq!(before, after);
    assert_eq!(
        serde_json::to_value(&manifest)?,
        serde_json::to_value(&reparsed)?
    );
    Ok(())
}

#[test]
fn six_native_components_keep_source_and_fixture_evidence() -> Result<(), Box<dyn std::error::Error>>
{
    let manifest = parse_manifest(DISPOSITION_JSON)?;
    let root = repo_root()?;
    let native_ids = registry_ids(NATIVE_RULES_JSON, "rules/cyberskills.json")?;
    let _adapter_ids = registry_ids(ADAPTER_RULES_JSON, "rules/cyberskills-adapters.json")?;
    let mut reviewed = 0;

    for record in manifest
        .records
        .iter()
        .filter(|record| record.decomposition_state == DecompositionState::Reviewed)
    {
        reviewed += 1;
        assert_eq!(record.components.len(), 1);
        let component = &record.components[0];
        assert_eq!(component.kind, ComponentKind::NativePredicate);
        assert_eq!(component.status, ComponentStatus::Implemented);
        let implementation = component
            .implementation_ref
            .as_ref()
            .ok_or("native component missing implementationRef")?;
        let executor_rule_id = implementation["executorRuleId"]
            .as_str()
            .ok_or("executorRuleId must be a string")?;
        assert_eq!(
            native_ids.get(executor_rule_id).map(String::as_str),
            Some(executor_rule_id)
        );
        assert_eq!(component.not_proved.len(), 1);
        for kind in [
            "source-attribution",
            "validator",
            "fail-fixture",
            "pass-fixture",
        ] {
            assert!(component
                .evidence_refs
                .iter()
                .any(|reference| reference["kind"] == kind));
        }
        let attribution_path = component
            .evidence_refs
            .iter()
            .find(|reference| reference["kind"] == "source-attribution")
            .ok_or("native component missing source attribution")?;
        let attribution_path = attribution_path["path"]
            .as_str()
            .ok_or("source attribution path must be a string")?;
        let attribution: Value =
            serde_json::from_str(&std::fs::read_to_string(root.join(attribution_path))?)?;
        assert_eq!(attribution["catalogId"], record.catalog_id);
        assert_eq!(
            attribution["sourceSha256"],
            record
                .source_sha256
                .as_deref()
                .ok_or("native record missing source hash")?
        );
        let source = std::fs::read(root.join(&record.source_path))?;
        assert_eq!(
            format!("{:x}", Sha256::digest(source)),
            record
                .source_sha256
                .as_deref()
                .ok_or("native record missing source hash")?
        );
        assert_eq!(component.evidence_refs.len(), 4);
    }
    assert_eq!(reviewed, 6);
    Ok(())
}

fn mutate(mut root: Value, case_name: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let records = root["records"]
        .as_array_mut()
        .ok_or("records must be an array")?;
    let reviewed = records
        .iter()
        .position(|record| record["catalogId"] == "exploiting-mass-assignment-in-rest-apis")
        .ok_or("reviewed fixture row missing")?;
    let unavailable = records
        .iter()
        .position(|record| record["catalogId"] == PROTECTED_CATALOG_ID)
        .ok_or("protected fixture row missing")?;
    match case_name {
        "duplicate-catalog-id" => records[1]["catalogId"] = records[0]["catalogId"].clone(),
        "duplicate-source-path" => records[1]["sourcePath"] = records[0]["sourcePath"].clone(),
        "empty-reviewed-components" => records[reviewed]["components"] = json!([]),
        "reviewed-missing-components" => {
            records[reviewed]
                .as_object_mut()
                .ok_or("reviewed row must be an object")?
                .remove("components");
        }
        "invalid-source-availability" => records[0]["sourceAvailability"] = json!("missing"),
        "invalid-decomposition-state" => records[0]["decompositionState"] = json!("blocked"),
        "invalid-component-kind" => records[reviewed]["components"][0]["kind"] = json!("guess"),
        "invalid-component-status" => records[reviewed]["components"][0]["status"] = json!("done"),
        "malformed-source-sha" => records[0]["sourceSha256"] = json!("ABC"),
        "unavailable-has-source-sha" => records[unavailable]["sourceSha256"] = json!("00"),
        "unavailable-has-components" => {
            records[unavailable]["components"] = json!([{"componentId":"x"}])
        }
        "mechanical-missing-predicate" => {
            records[reviewed]["components"][0]
                .as_object_mut()
                .ok_or("reviewed component must be an object")?
                .remove("predicate");
        }
        "mechanical-missing-not-proved" => {
            records[reviewed]["components"][0]
                .as_object_mut()
                .ok_or("reviewed component must be an object")?
                .remove("notProved");
        }
        "stale-totals" => root["totals"] = json!({"nativeMapped": 99}),
        "protected-blob-drift" => {
            records[unavailable]["unavailableSource"]["trackedBlob"] = json!("00")
        }
        other => return Err(format!("unknown fixture case: {other}").into()),
    }
    Ok(root)
}

#[test]
fn negative_fixture_matrix_rejects_contract_drift() -> Result<(), Box<dyn std::error::Error>> {
    let cases: Vec<Value> = serde_json::from_str(NEGATIVE_FIXTURES)?;
    let baseline: Value = serde_json::from_str(DISPOSITION_JSON)?;
    for case in cases {
        let name = case["name"].as_str().ok_or("fixture name missing")?;
        let mutated = serde_json::to_string(&mutate(baseline.clone(), name)?)?;
        assert!(
            parse_manifest(&mutated)
                .map(|manifest| validate_manifest(&manifest).is_err())
                .unwrap_or(true),
            "negative case unexpectedly accepted: {name}"
        );
    }
    Ok(())
}

proptest! {
    #[test]
    fn parser_rejects_or_accepts_arbitrary_utf8_without_panicking(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
        let raw = String::from_utf8_lossy(&bytes);
        let _ = parse_manifest(&raw);
    }
}
