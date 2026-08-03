//! Complete CP00 manifest validation.
//!
//! BOUNDARY-INVARIANT: the manifest is the single row-level validation entry.
//! NEGATIVE-TEST: schema, identity, and component drift is rejected here.

use std::collections::{BTreeMap, BTreeSet};

use super::components::{self, string_field};
use super::cp08_chain::validate_v3_record;
use super::source;
use super::wire;
use super::wire::cp08::{Cp08ProjectionDto, Cp08ProvenanceEntryDto};
use super::{require, DerivedDispositionCounts};

const EVIDENCE_KINDS: &[&str] = &[
    "source-attribution",
    "validator",
    "fail-fixture",
    "pass-fixture",
    "malformed-fixture",
    "boundary-fixture",
    "cli",
    "mcp",
    "ci",
    "adapter-recorded",
    "adapter-live",
    "manual-retention",
];

fn known_evidence_kind(value: &str) -> bool {
    EVIDENCE_KINDS.contains(&value)
}

pub(super) struct ChainEntryMap<'a> {
    pub(super) by_hash: BTreeMap<&'a str, &'a Cp08ProvenanceEntryDto>,
}

pub(super) fn validate_chain_entries<'a>(
    record: &wire::manifest::CyberSkillDispositionRecordDto,
    projection: &'a Cp08ProjectionDto,
    hashes: &mut BTreeSet<String>,
    successors: &mut BTreeMap<String, String>,
    correction_ids: &mut BTreeSet<String>,
) -> Result<ChainEntryMap<'a>, String> {
    let entries = projection
        .provenance_chain
        .iter()
        .map(|entry| (entry.artifact_sha256.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let mut results = Vec::new();
    for (index, entry) in projection.provenance_chain.iter().enumerate() {
        results.push(super::provenance::validate_entry_shape(
            record,
            entry,
            index,
            hashes,
            successors,
            correction_ids,
        ));
    }
    results.into_iter().find_map(Result::err).map_or_else(
        || {
            successors
                .iter()
                .find_map(|(prior, successor)| {
                    (!entries.contains_key(prior.as_str())
                        || !entries.contains_key(successor.as_str()))
                    .then_some(format!("orphan CP08 chain for {}", record.catalog_id))
                })
                .map_or(Ok(ChainEntryMap { by_hash: entries }), Err)
        },
        Err,
    )
}

fn validate_identity(
    record: &wire::manifest::CyberSkillDispositionRecordDto,
    ids: &mut BTreeSet<String>,
    paths: &mut BTreeSet<String>,
) -> Result<(), String> {
    let expected_path = format!(
        "vendor/anthropic-cybersecurity-skills/skills/{}/SKILL.md",
        record.catalog_id
    );
    require(record.source_path == expected_path, || {
        format!(
            "non-canonical sourcePath for {}: {}",
            record.catalog_id, record.source_path
        )
    })?;
    require(ids.insert(record.catalog_id.clone()), || {
        format!("duplicate catalogId: {}", record.catalog_id)
    })?;
    require(paths.insert(record.source_path.clone()), || {
        format!("duplicate sourcePath: {}", record.source_path)
    })?;
    require(
        string_field(&record.attribution, "sourceCatalog").is_ok()
            && string_field(&record.attribution, "vendor").is_ok()
            && string_field(&record.attribution, "sourcePath")
                .map(|value| value == record.source_path)
                .unwrap_or(false),
        || format!("attribution path drift for {}", record.catalog_id),
    )?;
    require(record.legacy_disposition.is_some(), || {
        format!("legacyDisposition missing for {}", record.catalog_id)
    })?;
    require(
        record
            .legacy_rationale
            .as_deref()
            .is_some_and(|reason| !reason.trim().is_empty()),
        || format!("legacyRationale missing for {}", record.catalog_id),
    )?;
    Ok(())
}

fn validate_component_set(
    record: &wire::manifest::CyberSkillDispositionRecordDto,
    counts: &mut DerivedDispositionCounts,
    component_ids: &mut BTreeSet<String>,
) -> Result<(), String> {
    for component in &record.components {
        components::validate_component(
            component,
            record,
            counts,
            component_ids,
            known_evidence_kind,
        )?;
    }
    Ok(())
}

fn validate_schema_record(
    schema_version: u32,
    record: &wire::manifest::CyberSkillDispositionRecordDto,
    counts: &mut DerivedDispositionCounts,
) -> Result<(), String> {
    match schema_version {
        2 => Ok(()),
        3 => validate_v3_record(record, counts),
        _ => Err(format!("unsupported schema version: {schema_version}")),
    }
}

pub(super) fn validate_manifest(
    manifest: &wire::manifest::CyberSkillsDispositionManifestDto,
) -> Result<DerivedDispositionCounts, String> {
    require(matches!(manifest.schema_version, 2 | 3), || {
        format!(
            "unsupported CyberSkills disposition schema version: {}",
            manifest.schema_version
        )
    })?;
    require(manifest.records.len() == 817, || {
        format!(
            "CyberSkills disposition must retain exactly 817 identities, got {}",
            manifest.records.len()
        )
    })?;
    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let mut component_ids = BTreeSet::new();
    let mut counts = DerivedDispositionCounts {
        identity_rows: manifest.records.len(),
        ..DerivedDispositionCounts::default()
    };
    for record in &manifest.records {
        validate_identity(record, &mut ids, &mut paths)?;
        validate_schema_record(manifest.schema_version, record, &mut counts)?;
        source::validate_source_state(record, &mut counts)?;
        validate_component_set(record, &mut counts, &mut component_ids)?;
    }
    require(counts.source_unavailable == 1, || {
        format!(
            "exactly one sourceUnavailable identity is required, got {}",
            counts.source_unavailable
        )
    })?;
    Ok(counts)
}
