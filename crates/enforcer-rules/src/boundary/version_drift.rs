//! Serde-only manifest ingress for version-drift detection.
//!
//! BOUNDARY-INVARIANT: manifest JSON never reaches drift comparison; the
//! comparison receives only validated typed manifest values.

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
use std::collections::BTreeMap;

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
    let schema_version = RuleManifestSchemaVersion::new(wire.schema_version)
        .map_err(|error| ManifestError::Invalid(super::super::boundary_reason(error)))?;
    let entries = wire
        .entries
        .into_iter()
        .map(|(id, entry)| {
            RuleVersion::new(entry.version)
                .map(|version| (id, RuleManifestEntry::new(version, entry.hash)))
                .map_err(|error| ManifestError::Invalid(super::super::boundary_reason(error)))
        })
        .collect::<Result<_, _>>()?;
    Ok(RuleManifest::new(schema_version, entries))
}

/// Render the pinned legacy manifest payload at the persistence boundary.
pub fn hash_record(record: &RuleRecord) -> Sha256 {
    let payload = format!("{{\"rule_id\":\"{}\",\"validator\":{{\"crateName\":\"{}\",\"path\":\"{}\"}},\"fixtures\":{{\"fail\":\"{}\",\"pass\":\"{}\"}},\"doc_anchor\":\"{}\"}}", record.rule_id, record.validator.crate_name.as_str(), record.validator.path.as_str(), record.fixtures.fail.as_str(), record.fixtures.pass.as_str(), record.doc_anchor.as_str());
    let digest = hash_chain::link_digest(None, payload.as_bytes());
    match Sha256::try_from(digest) {
        Ok(value) => value,
        Err(_) => unreachable!("link_digest emits a Sha256-compatible digest"),
    }
}
