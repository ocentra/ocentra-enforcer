//! Typed boundary for the CP09 cloud-security B07 manifest.
//!
//! BOUNDARY-INVARIANT: this decoder accepts only supplied offline references
//! for data classification, enclave trust, Security Hub, compliance, and
//! Defender records. It never connects to a provider, account, endpoint,
//! scanner, runtime, network, or production authority.
// NEGATIVE-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b07.rs
// ROUNDTRIP-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b07.rs

use std::collections::BTreeSet;

use serde::Deserialize;

const MACIE_SKILL: &str = "implementing-aws-macie-for-data-classification";
const NITRO_SKILL: &str = "implementing-aws-nitro-enclave-security";
const SECURITY_HUB_SKILL: &str = "implementing-aws-security-hub";
const COMPLIANCE_SKILL: &str = "implementing-aws-security-hub-compliance";
const DEFENDER_SKILL: &str = "implementing-azure-defender-for-cloud";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ManifestWire {
    schema_version: u8,
    bundle_id: String,
    owner: String,
    scope: String,
    evidence: Vec<EvidenceWire>,
    records: Vec<RecordWire>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EvidenceWire {
    kind: String,
    reference: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RecordWire {
    kind: String,
    skill_id: Option<String>,
    tenant_ref: Option<String>,
    account_ref: Option<String>,
    region_ref: Option<String>,
    resource_ref: Option<String>,
    policy_ref: Option<String>,
    classification_ref: Option<String>,
    data_boundary_ref: Option<String>,
    retention_ref: Option<String>,
    owner_ref: Option<String>,
    evidence_ref: Option<String>,
    authorization_ref: Option<String>,
    enclave_ref: Option<String>,
    attestation_ref: Option<String>,
    key_ref: Option<String>,
    scope_ref: Option<String>,
    finding_ref: Option<String>,
    severity_ref: Option<String>,
    status_ref: Option<String>,
    control_ref: Option<String>,
    standard_ref: Option<String>,
    compliance_ref: Option<String>,
    exception_ref: Option<String>,
    review_ref: Option<String>,
    subscription_ref: Option<String>,
    workspace_ref: Option<String>,
    defender_ref: Option<String>,
    alert_ref: Option<String>,
}

impl RecordWire {
    fn macie_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.tenant_ref.as_deref(),
            self.account_ref.as_deref(),
            self.resource_ref.as_deref(),
            self.classification_ref.as_deref(),
            self.data_boundary_ref.as_deref(),
            self.policy_ref.as_deref(),
            self.retention_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn nitro_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.account_ref.as_deref(),
            self.enclave_ref.as_deref(),
            self.attestation_ref.as_deref(),
            self.key_ref.as_deref(),
            self.policy_ref.as_deref(),
            self.authorization_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.scope_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn security_hub_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.account_ref.as_deref(),
            self.region_ref.as_deref(),
            self.workspace_ref.as_deref(),
            self.finding_ref.as_deref(),
            self.severity_ref.as_deref(),
            self.status_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.control_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn compliance_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.account_ref.as_deref(),
            self.region_ref.as_deref(),
            self.control_ref.as_deref(),
            self.standard_ref.as_deref(),
            self.compliance_ref.as_deref(),
            self.scope_ref.as_deref(),
            self.exception_ref.as_deref(),
            self.review_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn defender_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.tenant_ref.as_deref(),
            self.subscription_ref.as_deref(),
            self.workspace_ref.as_deref(),
            self.defender_ref.as_deref(),
            self.alert_ref.as_deref(),
            self.scope_ref.as_deref(),
            self.policy_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn schema(&self) -> Option<(&'static str, [Option<&str>; 10])> {
        match self.kind.as_str() {
            "aws-macie-data-classification" => Some((MACIE_SKILL, self.macie_refs())),
            "aws-nitro-enclave-security" => Some((NITRO_SKILL, self.nitro_refs())),
            "aws-security-hub" => Some((SECURITY_HUB_SKILL, self.security_hub_refs())),
            "aws-security-hub-compliance" => Some((COMPLIANCE_SKILL, self.compliance_refs())),
            "azure-defender-for-cloud" => Some((DEFENDER_SKILL, self.defender_refs())),
            _ => None,
        }
    }

    fn is_valid(&self) -> bool {
        let Some((expected_skill, required)) = self.schema() else {
            return false;
        };
        required.first().and_then(|value| *value) == Some(expected_skill)
            && required
                .iter()
                .skip(1)
                .all(|value| value.is_some_and(valid_ref))
    }
}

fn valid_ref(value: &str) -> bool {
    let Some((kind, identifier)) = value.split_once(':') else {
        return false;
    };
    !kind.is_empty()
        && !identifier.is_empty()
        && !value.chars().any(char::is_whitespace)
        && !kind.chars().any(char::is_whitespace)
}

fn valid_evidence(evidence: &[EvidenceWire]) -> bool {
    let mut seen = BTreeSet::new();
    !evidence.is_empty()
        && evidence.iter().all(|entry| {
            valid_ref(&entry.reference)
                && !entry.kind.trim().is_empty()
                && seen.insert(format!("{}:{}", entry.kind, entry.reference))
        })
}

fn valid_records(records: &[RecordWire]) -> bool {
    let mut kinds = BTreeSet::new();
    records.len() == 5
        && records
            .iter()
            .all(|record| kinds.insert(record.kind.clone()) && record.is_valid())
}

pub(crate) fn is_valid(source: &str) -> bool {
    let Ok(manifest) = serde_json::from_str::<ManifestWire>(source) else {
        return false;
    };
    manifest.schema_version == 1
        && !manifest.bundle_id.trim().is_empty()
        && !manifest.owner.trim().is_empty()
        && !manifest.scope.trim().is_empty()
        && valid_evidence(&manifest.evidence)
        && valid_records(&manifest.records)
}
