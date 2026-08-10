//! Typed boundary for the CP09 cloud-security B04 manifest.
//!
//! BOUNDARY-INVARIANT: this decoder accepts only supplied offline JSON
//! references for Azure service-principal, Azure storage, GuardDuty threat,
//! cloud-credential, and cloud-cryptomining records. It never connects to a
//! provider, log service, scanner, endpoint, tenant, network, or workload.
// NEGATIVE-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b04.rs
// ROUNDTRIP-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b04.rs

use std::collections::BTreeSet;

use serde::Deserialize;

const SERVICE_PRINCIPAL_SKILL: &str = "detecting-azure-service-principal-abuse";
const STORAGE_SKILL: &str = "detecting-azure-storage-account-misconfigurations";
const GUARDDUTY_SKILL: &str = "detecting-cloud-threats-with-guardduty";
const CREDENTIAL_SKILL: &str = "detecting-compromised-cloud-credentials";
const CRYPTOMINING_SKILL: &str = "detecting-cryptomining-in-cloud";

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
    identity_ref: Option<String>,
    principal_ref: Option<String>,
    credential_ref: Option<String>,
    role_ref: Option<String>,
    consent_ref: Option<String>,
    event_ref: Option<String>,
    storage_account_ref: Option<String>,
    access_ref: Option<String>,
    network_ref: Option<String>,
    encryption_ref: Option<String>,
    policy_ref: Option<String>,
    exception_ref: Option<String>,
    finding_ref: Option<String>,
    threat_ref: Option<String>,
    resource_ref: Option<String>,
    severity_ref: Option<String>,
    provenance_ref: Option<String>,
    session_ref: Option<String>,
    risk_ref: Option<String>,
    workload_ref: Option<String>,
    usage_ref: Option<String>,
    cost_ref: Option<String>,
    owner_ref: Option<String>,
    evidence_ref: Option<String>,
}

impl RecordWire {
    fn service_principal_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.tenant_ref.as_deref(),
            self.principal_ref.as_deref(),
            self.identity_ref.as_deref(),
            self.credential_ref.as_deref(),
            self.role_ref.as_deref(),
            self.consent_ref.as_deref(),
            self.event_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn storage_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.tenant_ref.as_deref(),
            self.storage_account_ref.as_deref(),
            self.access_ref.as_deref(),
            self.network_ref.as_deref(),
            self.encryption_ref.as_deref(),
            self.policy_ref.as_deref(),
            self.exception_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn guardduty_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.account_ref.as_deref(),
            self.finding_ref.as_deref(),
            self.threat_ref.as_deref(),
            self.resource_ref.as_deref(),
            self.severity_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
            self.provenance_ref.as_deref(),
            self.event_ref.as_deref(),
        ]
    }

    fn credential_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.account_ref.as_deref(),
            self.credential_ref.as_deref(),
            self.identity_ref.as_deref(),
            self.session_ref.as_deref(),
            self.event_ref.as_deref(),
            self.risk_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.severity_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn cryptomining_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.account_ref.as_deref(),
            self.workload_ref.as_deref(),
            self.resource_ref.as_deref(),
            self.identity_ref.as_deref(),
            self.usage_ref.as_deref(),
            self.cost_ref.as_deref(),
            self.finding_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn schema(&self) -> Option<(&'static str, [Option<&str>; 10])> {
        match self.kind.as_str() {
            "azure-service-principal-abuse" => {
                Some((SERVICE_PRINCIPAL_SKILL, self.service_principal_refs()))
            }
            "azure-storage-misconfiguration" => Some((STORAGE_SKILL, self.storage_refs())),
            "cloud-guardduty-threat" => Some((GUARDDUTY_SKILL, self.guardduty_refs())),
            "cloud-credential-compromise" => Some((CREDENTIAL_SKILL, self.credential_refs())),
            "cloud-cryptomining" => Some((CRYPTOMINING_SKILL, self.cryptomining_refs())),
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
