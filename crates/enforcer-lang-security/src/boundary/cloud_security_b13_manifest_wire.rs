//! Typed boundary for the CP09 cloud-security B13 manifest.
//!
//! BOUNDARY-INVARIANT: this decoder accepts only caller-supplied offline
//! references for IAM, Lambda, Azure Defender, container registry, and
//! managed Kubernetes security records. It never connects to a provider,
//! account, cluster, registry, function, scanner, runtime, network, or
//! production authority.
//!
//! NEGATIVE-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b13.rs
//! ROUNDTRIP-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b13.rs

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ManifestWire {
    pub(crate) schema_version: u8,
    pub(crate) bundle_id: String,
    pub(crate) owner: String,
    pub(crate) scope: String,
    pub(crate) evidence: Vec<EvidenceWire>,
    pub(crate) records: Vec<RecordWire>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EvidenceWire {
    pub(crate) kind: String,
    pub(crate) reference: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RecordWire {
    pub(crate) kind: String,
    pub(crate) skill_id: Option<String>,
    pub(crate) refs: Vec<String>,
}

/// Decode a supplied B13 manifest into typed wire records.
pub(crate) fn parse(source: &str) -> Result<ManifestWire, serde_json::Error> {
    serde_json::from_str::<ManifestWire>(source)
}
