//! Serde-only manifest ingress for version-drift detection.
//!
//! BOUNDARY-INVARIANT: manifest JSON never reaches drift comparison; the
//! comparison receives only validated typed manifest values.
//! boundaryOwnerNote: enforcer-rules owns version-manifest JSON decoding and hashing.
//! Negative invalid, empty, oversized, and malformed manifest coverage is exercised
//! by version-drift tests.

use super::super::{registry::RuleRecord, version_drift::ManifestError};
use enforcer_core::hash_chain;
use enforcer_domain::{
    hashes::Sha256,
    ids::RuleId,
    rules_types::{
        RuleManifest, RuleManifestEntry, RuleManifestJson, RuleManifestSchemaVersion, RuleVersion,
    },
};
use serde::Deserialize;
use std::{collections::BTreeMap, num::NonZeroU32};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireManifestEntry {
    version: u32,
    hash: Sha256,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireRegistryManifest {
    schema_version: u32,
    entries: BTreeMap<RuleId, WireManifestEntry>,
}

/// Decode and validate the version manifest at JSON ingress.
pub fn decode_manifest(raw: &RuleManifestJson) -> Result<RuleManifest, ManifestError> {
    let wire: WireRegistryManifest = serde_json::from_str(raw.as_str())
        .map_err(|error| ManifestError::Parse(super::super::boundary_reason(error)))?;
    let schema_version = RuleManifestSchemaVersion::try_new(
        NonZeroU32::new(wire.schema_version).ok_or_else(|| {
            ManifestError::Invalid(super::super::boundary_reason(
                "rule manifest schema version must be nonzero",
            ))
        })?,
    );
    let entries = wire
        .entries
        .into_iter()
        .map(|(id, entry)| {
            NonZeroU32::new(entry.version)
                .map(RuleVersion::try_new)
                .map(|version| (id, RuleManifestEntry::new(version, entry.hash)))
                .ok_or_else(|| {
                    ManifestError::Invalid(super::super::boundary_reason(
                        "rule version must be nonzero",
                    ))
                })
        })
        .collect::<Result<_, _>>()?;
    Ok(RuleManifest::new(schema_version, entries))
}

/// Render the pinned legacy manifest payload at the persistence boundary.
pub fn hash_record(record: &RuleRecord) -> Result<Sha256, ManifestError> {
    let payload = format!("{{\"rule_id\":\"{}\",\"validator\":{{\"crateName\":\"{}\",\"path\":\"{}\"}},\"fixtures\":{{\"fail\":\"{}\",\"pass\":\"{}\"}},\"doc_anchor\":\"{}\"}}", record.rule_id, record.validator.crate_name.as_str(), record.validator.path.as_str(), record.fixtures.fail.as_str(), record.fixtures.pass.as_str(), record.doc_anchor.as_str());
    let digest = hash_chain::link_digest(None, payload.as_bytes());
    Ok(digest)
}
