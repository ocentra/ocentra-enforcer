//! CP08 artifact recomputation and native-evidence tests.
//!
//! BOUNDARY-INVARIANT: these tests recompute immutable CP08 evidence without
//! reading vendor sources or promoting decomposition into implementation.
//! NEGATIVE-TEST: duplicate identities, kinds, hashes, and projection drift
//! are rejected by the parent negative matrix.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use enforcer_rules::cyberskills_disposition::{
    types::{
        ComponentKind, ComponentStatus, DecompositionState, ProjectionStatus, ProvenanceRelation,
        SourceAvailability,
    },
    wire::manifest::CyberSkillsDispositionManifestDto,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::support::{enum_wire_value, registry_ids, repo_root, sorted_strings};
use super::{
    parse_manifest, validate_manifest, ADAPTER_RULES_JSON, DISPOSITION_JSON, NATIVE_RULES_JSON,
};

#[test]
fn six_native_components_keep_source_and_fixture_evidence() -> Result<(), Box<dyn std::error::Error>>
{
    let manifest = parse_manifest(DISPOSITION_JSON)?;
    let root = repo_root()?;
    let native_ids = registry_ids(NATIVE_RULES_JSON, "rules/cyberskills.json")?;
    let _adapter_ids = registry_ids(ADAPTER_RULES_JSON, "rules/cyberskills-adapters.json")?;
    let mass_assignment = manifest
        .records
        .iter()
        .find(|record| record.catalog_id == "exploiting-mass-assignment-in-rest-apis")
        .ok_or("mass-assignment native row missing")?;
    assert_eq!(
        mass_assignment.source_anchors,
        vec![
            "\"role\":\"admin\"".to_owned(),
            "\"isAdmin\":true".to_owned(),
        ]
    );
    let reviewed = manifest
        .records
        .iter()
        .filter(|record| record.decomposition_state == DecompositionState::Reviewed)
        .map(|record| validate_native_record(&root, &native_ids, record))
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(reviewed.len(), 6);
    Ok(())
}

fn validate_native_record(
    root: &Path,
    native_ids: &BTreeSet<String>,
    record: &enforcer_rules::cyberskills_disposition::wire::manifest::CyberSkillDispositionRecordDto,
) -> Result<(), Box<dyn std::error::Error>> {
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
        .ok_or("native component missing source attribution")?["path"]
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
    assert_eq!(component.evidence_refs.len(), 4);
    assert_eq!(
        record
            .implementation
            .as_ref()
            .ok_or("native implementation projection missing")?
            .native
            .coverage,
        enforcer_rules::cyberskills_disposition::types::CoverageLevel::Complete
    );
    Ok(())
}

pub(crate) fn validate_cp08_artifacts(
    root: &Path,
    manifest: &CyberSkillsDispositionManifestDto,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut artifact_paths = std::fs::read_dir(root.join("proof/cyberskills/cp08"))?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("batch-"))
        .map(|entry| entry.path().join("decomposition.json"))
        .collect::<Vec<_>>();
    artifact_paths.sort();
    assert_eq!(artifact_paths.len(), 82);
    let records = manifest
        .records
        .iter()
        .map(|record| (record.catalog_id.as_str(), record))
        .collect::<BTreeMap<_, _>>();
    let mut stats = ArtifactStats::default();
    let mut ids = BTreeSet::new();
    for artifact_path in artifact_paths {
        validate_artifact(root, &artifact_path, &records, &mut ids, &mut stats)?;
    }
    assert_eq!(ids.len(), 816);
    assert_eq!(stats.components, 3206);
    assert_eq!(stats.complete, 758);
    assert_eq!(stats.partial, 58);
    assert_eq!(stats.missing_native, 56);
    assert_eq!(stats.missing_external, 2);
    assert_eq!(
        ids,
        records
            .values()
            .filter(|record| record.source_availability == SourceAvailability::Available)
            .map(|record| record.catalog_id.clone())
            .collect::<BTreeSet<_>>()
    );
    Ok(())
}

#[derive(Default)]
struct ArtifactStats {
    components: usize,
    complete: usize,
    partial: usize,
    missing_native: usize,
    missing_external: usize,
}

type RecordMap<'a> = BTreeMap<
    &'a str,
    &'a enforcer_rules::cyberskills_disposition::wire::manifest::CyberSkillDispositionRecordDto,
>;

type KindSummary = (
    BTreeMap<ComponentKind, ComponentStatus>,
    Vec<String>,
    Vec<String>,
);

struct ArtifactContext<'a> {
    batch: &'a str,
    relative_path: &'a str,
    artifact_sha: &'a str,
    records: &'a RecordMap<'a>,
}

struct ProjectionEvidence<'a> {
    missing: &'a [String],
    kinds: &'a [String],
    artifact_statuses: &'a BTreeMap<ComponentKind, ComponentStatus>,
    artifact: &'a ArtifactContext<'a>,
    source_sha: &'a str,
}

fn validate_artifact(
    root: &Path,
    artifact_path: &Path,
    records: &RecordMap<'_>,
    ids: &mut BTreeSet<String>,
    stats: &mut ArtifactStats,
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read(artifact_path)?;
    let artifact_sha = format!("{:x}", Sha256::digest(&bytes));
    let artifact: Value = serde_json::from_slice(&bytes)?;
    let artifact_batch = artifact["batch"]
        .as_str()
        .ok_or("CP08 batch missing batch")?;
    let relative_path = artifact_path
        .strip_prefix(root)?
        .to_string_lossy()
        .replace('\\', "/");
    let context = ArtifactContext {
        batch: artifact_batch,
        relative_path: &relative_path,
        artifact_sha: &artifact_sha,
        records,
    };
    for skill in artifact["skills"].as_array().ok_or("CP08 skills missing")? {
        validate_skill(skill, &context, ids, stats)?;
    }
    Ok(())
}

fn validate_skill(
    skill: &Value,
    artifact: &ArtifactContext<'_>,
    ids: &mut BTreeSet<String>,
    stats: &mut ArtifactStats,
) -> Result<(), Box<dyn std::error::Error>> {
    let catalog_id = skill["catalogId"]
        .as_str()
        .ok_or("CP08 catalogId missing")?;
    assert!(
        ids.insert(catalog_id.to_owned()),
        "duplicate CP08 ID: {catalog_id}"
    );
    let record = artifact
        .records
        .get(catalog_id)
        .ok_or_else(|| format!("CP08 ID missing from ledger: {catalog_id}"))?;
    assert_eq!(record.source_availability, SourceAvailability::Available);
    let source_sha = skill["source"]["sha256"]
        .as_str()
        .ok_or("CP08 source SHA missing")?;
    assert_eq!(record.source_sha256.as_deref(), Some(source_sha));
    let components = skill["components"]
        .as_array()
        .ok_or("CP08 components missing")?;
    let (artifact_statuses, kinds, missing) = collect_kinds(components)?;
    assert_component_evidence(components);
    let projection = record
        .cp08_projection
        .as_ref()
        .ok_or("CP08 projection missing from ledger")?;
    let evidence = ProjectionEvidence {
        missing: &missing,
        kinds: &kinds,
        artifact_statuses: &artifact_statuses,
        artifact,
        source_sha,
    };
    assert_projection_evidence(skill, record, projection, &evidence)?;
    stats.components += components.len();
    stats.complete += usize::from(missing.is_empty());
    stats.partial += usize::from(!missing.is_empty());
    stats.missing_native += usize::from(missing.iter().any(|kind| kind == "native-predicate"));
    stats.missing_external += usize::from(missing.iter().any(|kind| kind == "external-engine"));
    Ok(())
}

fn collect_kinds(components: &[Value]) -> Result<KindSummary, Box<dyn std::error::Error>> {
    let statuses = components
        .iter()
        .map(|component| {
            Ok::<_, Box<dyn std::error::Error>>((
                serde_json::from_value(component["kind"].clone())?,
                serde_json::from_value(component["status"].clone())?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let kinds = components
        .iter()
        .map(|component| component["kind"].as_str().unwrap_or_default().to_owned())
        .collect::<Vec<_>>();
    let unique = kinds.iter().cloned().collect::<BTreeSet<_>>();
    assert_eq!(unique.len(), kinds.len(), "duplicate CP08 kind");
    assert!(unique.iter().all(|kind| [
        "native-predicate",
        "external-engine",
        "advisory",
        "manual"
    ]
    .contains(&kind.as_str())));
    let missing = ["native-predicate", "external-engine", "advisory", "manual"]
        .iter()
        .filter(|kind| !unique.contains(**kind))
        .map(|kind| (*kind).to_owned())
        .collect::<Vec<_>>();
    Ok((statuses, kinds, missing))
}

fn assert_component_evidence(components: &[Value]) {
    components.iter().for_each(|component| {
        assert!(!component["notProved"]
            .as_array()
            .unwrap_or(&vec![])
            .is_empty())
    });
}

fn assert_projection_evidence(
    skill: &Value,
    record: &enforcer_rules::cyberskills_disposition::wire::manifest::CyberSkillDispositionRecordDto,
    projection: &enforcer_rules::cyberskills_disposition::wire::cp08::Cp08ProjectionDto,
    evidence: &ProjectionEvidence<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let ProjectionEvidence {
        missing,
        kinds,
        artifact_statuses,
        artifact,
        source_sha,
    } = evidence;
    assert_eq!(projection.component_count, kinds.len());
    assert_eq!(
        sorted_strings(projection.present_kinds.iter().map(enum_wire_value)),
        sorted_strings(kinds.iter().cloned())
    );
    assert_eq!(
        sorted_strings(
            projection
                .missing_kinds
                .iter()
                .cloned()
                .map(|kind| enum_wire_value(&kind))
        ),
        sorted_strings(missing.iter().cloned())
    );
    assert_eq!(
        projection.status,
        if missing.is_empty() {
            ProjectionStatus::Complete
        } else {
            ProjectionStatus::Partial
        }
    );
    assert_eq!(&projection.kind_status, *artifact_statuses);
    assert_eq!(projection.provenance_chain.len(), 1);
    let provenance = &projection.provenance_chain[0];
    assert_eq!(provenance.relation, ProvenanceRelation::Accepted);
    assert_eq!(provenance.batch.as_str(), artifact.batch);
    assert_eq!(provenance.artifact_path.as_str(), artifact.relative_path);
    assert_eq!(provenance.artifact_sha256.as_str(), artifact.artifact_sha);
    assert_eq!(provenance.source_sha256.as_str(), *source_sha);
    let artifact_anchors: Vec<String> = serde_json::from_value(skill["source"]["anchors"].clone())?;
    assert_eq!(
        provenance
            .artifact_anchors
            .iter()
            .map(|anchor| anchor.as_str().to_owned())
            .collect::<Vec<_>>(),
        artifact_anchors
    );
    assert_ne!(artifact_anchors, record.source_anchors);
    Ok(())
}

#[test]
fn cp08_artifacts_recompute_verified_projection_without_vendor_reads(
) -> Result<(), Box<dyn std::error::Error>> {
    let manifest = parse_manifest(DISPOSITION_JSON)?;
    validate_manifest(&manifest)?;
    validate_cp08_artifacts(&repo_root()?, &manifest)
}
