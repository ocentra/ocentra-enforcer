//! Typed boundary for the CP09 compliance-governance B02 manifest.
//!
//! BOUNDARY-INVARIANT: this decoder accepts only caller-supplied offline
//! framework, vendor, control, maturity, and evidence records. It never
//! connects to a regulator, auditor, vendor portal, cloud service, endpoint,
//! payment system, or production authority.
// NEGATIVE-TEST: crates/enforcer-lang-security/tests/cyberskills_compliance_governance_manifest_b02.rs
// ROUNDTRIP-TEST: crates/enforcer-lang-security/tests/cyberskills_compliance_governance_manifest_b02.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ManifestWire {
    pub(crate) schema_version: u8,
    pub(crate) bundle_id: String,
    pub(crate) owner: String,
    pub(crate) scope: String,
    pub(crate) evidence: Vec<EvidenceWire>,
    pub(crate) records: Vec<RecordWire>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EvidenceWire {
    pub(crate) kind: String,
    pub(crate) reference: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RecordWire {
    pub(crate) kind: String,
    pub(crate) skill_id: Option<String>,
    pub(crate) refs: Vec<String>,
    pub(crate) controls: Option<Vec<ControlWire>>,
    pub(crate) documents: Option<Vec<DocumentWire>>,
    pub(crate) risks: Option<Vec<RiskWire>>,
    pub(crate) vendor_profiles: Option<Vec<VendorProfileWire>>,
    pub(crate) maturity_items: Option<Vec<MaturityWire>>,
    pub(crate) evidence_items: Option<Vec<EvidenceItemWire>>,
    pub(crate) readiness: Option<Vec<ReadinessWire>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ControlWire {
    pub(crate) id: String,
    pub(crate) family: String,
    pub(crate) status: String,
    pub(crate) requirement: String,
    pub(crate) evidence_reference: Option<String>,
    pub(crate) justification: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DocumentWire {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) status: String,
    pub(crate) last_reviewed: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RiskWire {
    pub(crate) id: String,
    pub(crate) level: String,
    pub(crate) treatment: String,
    pub(crate) owner: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct VendorProfileWire {
    pub(crate) vendor_id: String,
    pub(crate) data_sensitivity: String,
    pub(crate) access: String,
    pub(crate) criticality: String,
    pub(crate) regulated_scope: bool,
    pub(crate) integration: String,
    pub(crate) concentration: bool,
    pub(crate) risk_score: u8,
    pub(crate) tier: String,
    pub(crate) evidence: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MaturityWire {
    pub(crate) category: String,
    pub(crate) score: u8,
    pub(crate) target: u8,
    pub(crate) evidence_reference: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EvidenceItemWire {
    pub(crate) id: String,
    pub(crate) control_id: String,
    pub(crate) status: String,
    pub(crate) period_start: String,
    pub(crate) period_end: String,
    pub(crate) reference: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ReadinessWire {
    pub(crate) id: String,
    pub(crate) area: String,
    pub(crate) status: bool,
    pub(crate) owner: String,
}

/// Decode one supplied B02 compliance-governance manifest into typed records.
pub(crate) fn parse(source: &str) -> Result<ManifestWire, serde_json::Error> {
    serde_json::from_str::<ManifestWire>(source)
}
