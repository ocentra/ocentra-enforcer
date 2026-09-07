//! Private CP08 validation dispatch.
//!
//! BOUNDARY-INVARIANT: dispatch validates typed records before derived counts.
//! NEGATIVE-TEST: chain, source-availability, and projection mutations fail.
//! ROUNDTRIP-TEST: crates/enforcer-rules/tests/cyberskills_disposition/cp08.rs

use std::collections::{BTreeMap, BTreeSet};

use super::types::Sha256ValueEnvelope;
use super::types::SourceAvailability;
use super::wire::cp08::{Cp08ProjectionDto, Cp08ProvenanceEntryDto};
use super::wire::manifest::CyberSkillDispositionRecordDto;
use super::DerivedDispositionCounts;

pub(super) fn validate_v3_record(
    record: &CyberSkillDispositionRecordDto,
    counts: &mut DerivedDispositionCounts,
) -> Result<(), String> {
    super::source::validate_source_projection(record)?;
    match record.source_availability {
        SourceAvailability::Available => {
            super::implementation_truth::validate_available_cp08(record, counts)?
        }
        SourceAvailability::SourceUnavailable => {
            super::source::validate_unavailable_identity(record)?
        }
    }
    super::implementation_truth::validate_implementation(record)
}

pub(super) fn validate_chain(
    record: &CyberSkillDispositionRecordDto,
    projection: &Cp08ProjectionDto,
) -> Result<(), String> {
    super::provenance::validate_chain(record, projection)
}

fn first_error<const N: usize>(checks: [Result<(), String>; N]) -> Result<(), String> {
    checks.into_iter().find_map(Result::err).map_or(Ok(()), Err)
}

fn all_true<const N: usize>(checks: [bool; N]) -> bool {
    checks.into_iter().all(std::convert::identity)
}

struct ChainWalk<'a> {
    entries: &'a BTreeMap<&'a str, &'a Cp08ProvenanceEntryDto>,
    successors: &'a BTreeMap<String, String>,
    reached: &'a mut BTreeSet<String>,
    catalog_id: &'a str,
}

pub(super) fn walk_chain(
    record: &CyberSkillDispositionRecordDto,
    projection: &Cp08ProjectionDto,
    successors: &BTreeMap<String, String>,
    hashes: &BTreeSet<String>,
    entries: &BTreeMap<&str, &Cp08ProvenanceEntryDto>,
) -> Result<(), String> {
    let root = projection
        .provenance_chain
        .first()
        .ok_or_else(|| format!("CP08 provenance chain empty: {}", record.catalog_id))?;
    let mut reached = BTreeSet::new();
    let mut walk = ChainWalk {
        entries,
        successors,
        reached: &mut reached,
        catalog_id: record.catalog_id.as_str(),
    };
    let terminal_hash = follow_chain(root, root.artifact_sha256.as_str().to_owned(), &mut walk)?;
    let terminal = entries
        .get(terminal_hash.as_str())
        .ok_or_else(|| format!("terminal CP08 provenance missing: {}", record.catalog_id))?;
    super::ensure(
        reached.len() == hashes.len(),
        format!(
            "orphan or forked CP08 provenance chain: {}",
            record.catalog_id
        ),
    )?;
    super::ensure(
        all_true([
            projection.component_count == terminal.component_count,
            projection.present_kinds == terminal.present_kinds,
            projection.missing_kinds == terminal.missing_kinds,
            projection.kind_status == terminal.kind_status,
        ]),
        format!("CP08 projection cache drift: {}", record.catalog_id),
    )
}

fn follow_chain(
    current: &Cp08ProvenanceEntryDto,
    current_hash: String,
    walk: &mut ChainWalk<'_>,
) -> Result<String, String> {
    super::ensure(
        walk.reached.insert(current_hash.clone()),
        format!("cycle in CP08 provenance chain: {}", walk.catalog_id),
    )?;
    let Some(next_hash) = walk.successors.get(&current_hash) else {
        return Ok(current_hash);
    };
    let next = walk
        .entries
        .get(next_hash.as_str())
        .ok_or_else(|| format!("orphan CP08 provenance chain: {}", walk.catalog_id))?;
    validate_transition(current, next, &current_hash, walk.catalog_id)?;
    follow_chain(next, next_hash.clone(), walk)
}

fn validate_transition(
    current: &Cp08ProvenanceEntryDto,
    next: &Cp08ProvenanceEntryDto,
    current_hash: &str,
    catalog_id: &str,
) -> Result<(), String> {
    let present = current
        .present_kinds
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let added = next.adds_kinds.iter().copied().collect::<BTreeSet<_>>();
    first_error([
        super::ensure(
            all_true([
                added.len() == next.adds_kinds.len(),
                present.is_disjoint(&added),
            ]),
            format!("non-monotonic CP08 correction: {}", catalog_id),
        ),
        super::ensure(
            next.prior_artifact_sha256
                .as_ref()
                .map(Sha256ValueEnvelope::as_str)
                == Some(current_hash),
            format!("correction predecessor drift: {}", catalog_id),
        ),
        super::ensure(
            present.union(&added).copied().collect::<BTreeSet<_>>()
                == next.present_kinds.iter().copied().collect::<BTreeSet<_>>(),
            format!("CP08 correction kind merge drift: {}", catalog_id),
        ),
    ])
}
