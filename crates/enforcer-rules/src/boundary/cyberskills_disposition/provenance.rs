//! CP08 provenance value ownership.
//!
//! BOUNDARY-INVARIANT: provenance values are role-specific DTO fields; chain
//! relations are validated by the manifest integrity phase.
//! NEGATIVE-TEST: malformed provenance is rejected by the chain validator.

use std::collections::{BTreeMap, BTreeSet};

use super::projection::validate_projection_snapshot;
use super::types::ProvenanceRelation;
use super::wire::cp08::{Cp08ProjectionDto, Cp08ProvenanceEntryDto};
use super::wire::manifest::CyberSkillDispositionRecordDto;

fn first_error<const N: usize>(checks: [Result<(), String>; N]) -> Result<(), String> {
    checks.into_iter().find_map(Result::err).map_or(Ok(()), Err)
}

fn all_true<const N: usize>(checks: [bool; N]) -> bool {
    checks.into_iter().all(std::convert::identity)
}

#[derive(Default)]
pub(super) struct EntryValidationState {
    pub(super) hashes: BTreeSet<String>,
    pub(super) successors: BTreeMap<String, String>,
    correction_ids: BTreeSet<String>,
}

pub(super) fn validate_entry_shape(
    record: &CyberSkillDispositionRecordDto,
    entry: &Cp08ProvenanceEntryDto,
    index: usize,
    state: &mut EntryValidationState,
) -> Result<(), String> {
    first_error([
        super::ensure(
            entry.source_sha256.as_str() == record.source_sha256.as_deref().unwrap_or_default(),
            format!("CP08/source hash mismatch: {}", record.catalog_id),
        ),
        super::source::validate_artifact_anchors(
            record.source_anchors.as_slice(),
            entry.artifact_anchors.as_slice(),
        ),
        validate_projection_snapshot(
            entry.component_count,
            &entry.present_kinds,
            &entry.missing_kinds,
            &entry.kind_status,
        ),
        super::ensure(
            state
                .hashes
                .insert(entry.artifact_sha256.as_str().to_owned()),
            format!("duplicate CP08 artifact hash: {}", record.catalog_id),
        ),
        validate_relation(
            entry,
            index,
            &mut state.correction_ids,
            &mut state.successors,
            record.catalog_id.as_str(),
        ),
    ])
}

fn validate_relation(
    entry: &Cp08ProvenanceEntryDto,
    index: usize,
    correction_ids: &mut BTreeSet<String>,
    successors: &mut BTreeMap<String, String>,
    catalog_id: &str,
) -> Result<(), String> {
    if entry.relation == ProvenanceRelation::Accepted {
        super::ensure(
            all_true([
                index == 0,
                entry.prior_artifact_sha256.is_none(),
                entry.correction_id.is_none(),
                entry.adds_kinds.is_empty(),
            ]),
            format!("invalid CP08 provenance root: {}", catalog_id),
        )
    } else {
        validate_correction(entry, correction_ids, successors, catalog_id)
    }
}

fn validate_correction(
    entry: &Cp08ProvenanceEntryDto,
    correction_ids: &mut BTreeSet<String>,
    successors: &mut BTreeMap<String, String>,
    catalog_id: &str,
) -> Result<(), String> {
    let prior = entry
        .prior_artifact_sha256
        .as_ref()
        .ok_or(format!("correction prior missing for {catalog_id}"))?;
    let correction_id = entry
        .correction_id
        .as_ref()
        .ok_or(format!("correction ID missing for {catalog_id}"))?;
    first_error([
        super::ensure(
            !entry.adds_kinds.is_empty(),
            format!("correction adds no kinds for {catalog_id}"),
        ),
        super::ensure(
            correction_ids.insert(correction_id.as_str().to_owned()),
            format!("duplicate correction ID for {catalog_id}"),
        ),
        super::ensure(
            successors
                .insert(
                    prior.as_str().to_owned(),
                    entry.artifact_sha256.as_str().to_owned(),
                )
                .is_none(),
            format!("correction successor fork for {catalog_id}"),
        ),
    ])
}

pub(super) fn validate_chain(
    record: &CyberSkillDispositionRecordDto,
    projection: &Cp08ProjectionDto,
) -> Result<(), String> {
    let mut state = EntryValidationState::default();
    let chain = super::manifest::validate_chain_entries(record, projection, &mut state);
    chain.map_or_else(Err, |entries| {
        super::cp08_chain::walk_chain(
            record,
            projection,
            &state.successors,
            &state.hashes,
            &entries.by_hash,
        )
    })
}
