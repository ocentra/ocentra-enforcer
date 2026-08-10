//! Typed boundary for the CP09 compliance-governance B01 manifest.
//!
//! BOUNDARY-INVARIANT: this decoder accepts only caller-supplied offline
//! control, risk, authorization, privacy, and safeguard records. It never
//! connects to a framework authority, assessor, regulator, GRC service,
//! production system, personal-data store, or healthcare authority.
// NEGATIVE-TEST: crates/enforcer-lang-security/tests/cyberskills_compliance_governance_manifest_b01.rs
// ROUNDTRIP-TEST: crates/enforcer-lang-security/tests/cyberskills_compliance_governance_manifest_b01.rs

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
    pub(crate) score: Option<i32>,
    pub(crate) risk_items: Option<Vec<RiskWire>>,
    pub(crate) information_types: Option<Vec<InformationTypeWire>>,
    pub(crate) categorization: Option<String>,
    pub(crate) processing_activities: Option<Vec<ProcessingActivityWire>>,
    pub(crate) data_subject_requests: Option<Vec<DataSubjectRequestWire>>,
    pub(crate) breach_records: Option<Vec<BreachRecordWire>>,
    pub(crate) safeguards: Option<Vec<SafeguardWire>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ControlWire {
    pub(crate) id: String,
    pub(crate) family: String,
    pub(crate) status: String,
    pub(crate) weight: Option<u8>,
    pub(crate) partial_deduction: Option<u8>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RiskWire {
    pub(crate) id: String,
    pub(crate) threat_event: String,
    pub(crate) asset: String,
    pub(crate) likelihood: String,
    pub(crate) impact: String,
    pub(crate) risk_level: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct InformationTypeWire {
    pub(crate) name: String,
    pub(crate) confidentiality: String,
    pub(crate) integrity: String,
    pub(crate) availability: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProcessingActivityWire {
    pub(crate) activity_id: String,
    pub(crate) purpose: String,
    pub(crate) lawful_basis: String,
    pub(crate) data_categories: Vec<String>,
    pub(crate) data_subjects: Vec<String>,
    pub(crate) recipients: Vec<String>,
    pub(crate) retention_period: String,
    pub(crate) security_measures: Vec<String>,
    pub(crate) international_transfers: Vec<String>,
    pub(crate) dpia_required: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DataSubjectRequestWire {
    pub(crate) id: String,
    pub(crate) request_type: String,
    pub(crate) received_date: String,
    pub(crate) deadline: String,
    pub(crate) status: String,
    pub(crate) identity_verified: bool,
    pub(crate) days_elapsed: u32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct BreachRecordWire {
    pub(crate) id: String,
    pub(crate) detected_at: String,
    pub(crate) severity: String,
    pub(crate) subjects_affected: u64,
    pub(crate) authority_notified: bool,
    pub(crate) subjects_notified: bool,
    pub(crate) notification_hours: Option<u16>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SafeguardWire {
    pub(crate) id: String,
    pub(crate) section: String,
    pub(crate) name: String,
    pub(crate) requirement: String,
    pub(crate) status: String,
    pub(crate) alternative_documented: Option<bool>,
}

/// Decode one supplied B01 compliance-governance manifest into typed records.
pub(crate) fn parse(source: &str) -> Result<ManifestWire, serde_json::Error> {
    serde_json::from_str::<ManifestWire>(source)
}
