//! Typed boundary for the CP09 cloud-security B09 manifest.
//!
//! BOUNDARY-INVARIANT: this decoder accepts only supplied offline references
//! for binary authorization, organization policy, VPC firewall, Vault, and
//! cloud zero-trust records. It never connects to a provider, account,
//! endpoint, scanner, runtime, network, or production authority.
// NEGATIVE-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b09.rs
// ROUNDTRIP-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b09.rs

use std::collections::BTreeSet;

use serde::Deserialize;

const BINARY_AUTH_SKILL: &str = "implementing-gcp-binary-authorization";
const ORG_POLICY_SKILL: &str = "implementing-gcp-organization-policy-constraints";
const FIREWALL_SKILL: &str = "implementing-gcp-vpc-firewall-rules";
const VAULT_SKILL: &str = "implementing-secrets-management-with-vault";
const ZERO_TRUST_SKILL: &str = "implementing-zero-trust-in-cloud";

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
    project_ref: Option<String>,
    organization_ref: Option<String>,
    folder_ref: Option<String>,
    location_ref: Option<String>,
    attestor_ref: Option<String>,
    image_ref: Option<String>,
    policy_ref: Option<String>,
    signature_ref: Option<String>,
    enforcement_ref: Option<String>,
    constraint_ref: Option<String>,
    condition_ref: Option<String>,
    exception_ref: Option<String>,
    network_ref: Option<String>,
    subnet_ref: Option<String>,
    rule_ref: Option<String>,
    direction_ref: Option<String>,
    target_ref: Option<String>,
    action_ref: Option<String>,
    namespace_ref: Option<String>,
    mount_ref: Option<String>,
    secret_ref: Option<String>,
    role_ref: Option<String>,
    key_ref: Option<String>,
    lease_ref: Option<String>,
    tenant_ref: Option<String>,
    identity_ref: Option<String>,
    device_ref: Option<String>,
    segment_ref: Option<String>,
    access_ref: Option<String>,
    session_ref: Option<String>,
    owner_ref: Option<String>,
    evidence_ref: Option<String>,
}

impl RecordWire {
    fn binary_authorization_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.project_ref.as_deref(),
            self.location_ref.as_deref(),
            self.attestor_ref.as_deref(),
            self.image_ref.as_deref(),
            self.policy_ref.as_deref(),
            self.signature_ref.as_deref(),
            self.enforcement_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn organization_policy_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.organization_ref.as_deref(),
            self.folder_ref.as_deref(),
            self.project_ref.as_deref(),
            self.constraint_ref.as_deref(),
            self.policy_ref.as_deref(),
            self.condition_ref.as_deref(),
            self.exception_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn firewall_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.project_ref.as_deref(),
            self.network_ref.as_deref(),
            self.subnet_ref.as_deref(),
            self.rule_ref.as_deref(),
            self.direction_ref.as_deref(),
            self.target_ref.as_deref(),
            self.action_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn vault_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.namespace_ref.as_deref(),
            self.mount_ref.as_deref(),
            self.secret_ref.as_deref(),
            self.policy_ref.as_deref(),
            self.role_ref.as_deref(),
            self.key_ref.as_deref(),
            self.lease_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn zero_trust_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.tenant_ref.as_deref(),
            self.identity_ref.as_deref(),
            self.device_ref.as_deref(),
            self.policy_ref.as_deref(),
            self.segment_ref.as_deref(),
            self.access_ref.as_deref(),
            self.session_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn schema(&self) -> Option<(&'static str, [Option<&str>; 10])> {
        match self.kind.as_str() {
            "gcp-binary-authorization" => {
                Some((BINARY_AUTH_SKILL, self.binary_authorization_refs()))
            }
            "gcp-organization-policy-constraints" => {
                Some((ORG_POLICY_SKILL, self.organization_policy_refs()))
            }
            "gcp-vpc-firewall-rules" => Some((FIREWALL_SKILL, self.firewall_refs())),
            "vault-secrets-management" => Some((VAULT_SKILL, self.vault_refs())),
            "zero-trust-cloud" => Some((ZERO_TRUST_SKILL, self.zero_trust_refs())),
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
