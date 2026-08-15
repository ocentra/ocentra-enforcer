//! Focused CP00 integration tests.

use std::collections::BTreeSet;
use std::path::PathBuf;

use super::support::source_catalog_ids;
use super::{
    parse_manifest, validate_manifest, DISPOSITION_JSON, PROTECTED_CATALOG_ID,
    PROTECTED_SOURCE_PATH, PROTECTED_TRACKED_BLOB,
};
use enforcer_domain::rules_types::{RuleCatalogJson, RuleCatalogSource};
use enforcer_rules::cyberskills_disposition::types::{
    CoverageLevel, DecompositionState, SourceAvailability,
};
use enforcer_rules::cyberskills_disposition::wire::cp08::{
    Cp08ProjectionDto, Cp08ProvenanceEntryDto, SourceIdentityDto,
};
use enforcer_rules::cyberskills_disposition::wire::implementation::{
    ExecutableProofDto, ImplementationTruthDto, NativeImplementationDto,
};
use enforcer_rules::cyberskills_disposition::wire::manifest::{
    CyberSkillComponentDto, CyberSkillDispositionRecordDto, CyberSkillsDispositionManifestDto,
};
use enforcer_rules::loader::parse_catalog;

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
    assert_eq!(counts.cp08_complete_rows, 758);
    assert_eq!(counts.cp08_partial_rows, 58);
    assert_eq!(counts.cp08_component_count, 3206);
    assert_eq!(counts.cp08_missing_native, 56);
    assert_eq!(counts.cp08_missing_external, 2);
    assert_eq!(source_catalog_ids().len(), 817);
    assert_eq!(
        manifest
            .records
            .iter()
            .map(|record| record.catalog_id.as_str())
            .collect::<BTreeSet<_>>(),
        source_catalog_ids().iter().map(String::as_str).collect()
    );
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
    assert!(unavailable.cp08_projection.is_none());
    assert_eq!(
        unavailable
            .implementation
            .as_ref()
            .ok_or("protected implementation projection missing")?
            .native
            .coverage,
        CoverageLevel::None
    );
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
fn v3_dto_and_role_newtypes_round_trip_with_validation() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = parse_manifest(DISPOSITION_JSON)?;
    let manifest_wire = serde_json::to_vec(&manifest)?;
    let manifest_round_trip: CyberSkillsDispositionManifestDto =
        serde_json::from_slice(&manifest_wire)?;
    assert_eq!(manifest_round_trip, manifest);
    let record = manifest
        .records
        .iter()
        .find(|record| record.cp08_projection.is_some())
        .ok_or("CP08 row missing")?;
    let record_wire = serde_json::to_vec(record)?;
    let record_round_trip: CyberSkillDispositionRecordDto = serde_json::from_slice(&record_wire)?;
    assert_eq!(&record_round_trip, record);
    let component_record = manifest
        .records
        .iter()
        .find(|candidate| !candidate.components.is_empty())
        .ok_or("native component record missing")?;
    let component = component_record
        .components
        .first()
        .ok_or("native component missing")?;
    let component_wire = serde_json::to_vec(component)?;
    let component_round_trip: CyberSkillComponentDto = serde_json::from_slice(&component_wire)?;
    assert_eq!(&component_round_trip, component);
    let source = record.source.as_ref().ok_or("source projection missing")?;
    let projection = record
        .cp08_projection
        .as_ref()
        .ok_or("CP08 projection missing")?;
    let provenance = projection
        .provenance_chain
        .first()
        .ok_or("CP08 provenance missing")?;
    let implementation = record
        .implementation
        .as_ref()
        .ok_or("implementation truth missing")?;

    let source_wire = serde_json::to_vec(source)?;
    let source_round_trip: SourceIdentityDto = serde_json::from_slice(&source_wire)?;
    assert_eq!(&source_round_trip, source);

    let projection_wire = serde_json::to_vec(projection)?;
    let projection_round_trip: Cp08ProjectionDto = serde_json::from_slice(&projection_wire)?;
    assert_eq!(&projection_round_trip, projection);

    let provenance_wire = serde_json::to_vec(provenance)?;
    let provenance_round_trip: Cp08ProvenanceEntryDto = serde_json::from_slice(&provenance_wire)?;
    assert_eq!(&provenance_round_trip, provenance);

    let implementation_wire = serde_json::to_vec(implementation)?;
    let implementation_round_trip: ImplementationTruthDto =
        serde_json::from_slice(&implementation_wire)?;
    assert_eq!(&implementation_round_trip, implementation);

    let native_wire = serde_json::to_vec(&implementation.native)?;
    let native_round_trip: NativeImplementationDto = serde_json::from_slice(&native_wire)?;
    assert_eq!(&native_round_trip, &implementation.native);

    let proof_wire = serde_json::to_vec(&implementation.executable_proof)?;
    let proof_round_trip: ExecutableProofDto = serde_json::from_slice(&proof_wire)?;
    assert_eq!(&proof_round_trip, &implementation.executable_proof);
    Ok(())
}
