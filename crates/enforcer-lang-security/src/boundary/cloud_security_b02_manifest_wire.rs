//! Typed boundary for the CP09 cloud-security B02 manifest.
//!
//! BOUNDARY-INVARIANT: this decoder accepts only supplied offline JSON
//! references for GCP IAM, Kubernetes RBAC, Terraform security, cloud SIEM,
//! and authorized cloud-assessment records. It never connects to a provider,
//! cluster, state backend, tenant, scanner, endpoint, or production system.
// NEGATIVE-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b02.rs
// ROUNDTRIP-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b02.rs

use std::collections::BTreeSet;

use serde::Deserialize;

const GCP_IAM_SKILL: &str = "auditing-gcp-iam-permissions";
const KUBERNETES_RBAC_SKILL: &str = "auditing-kubernetes-cluster-rbac";
const TERRAFORM_SKILL: &str = "auditing-terraform-infrastructure-for-security";
const CLOUD_SIEM_SKILL: &str = "building-cloud-siem-with-sentinel";
const CLOUD_PENTEST_SKILL: &str = "conducting-cloud-penetration-testing";

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
    principal_ref: Option<String>,
    binding_ref: Option<String>,
    role_ref: Option<String>,
    key_ref: Option<String>,
    cluster_ref: Option<String>,
    namespace_ref: Option<String>,
    subject_ref: Option<String>,
    verb_ref: Option<String>,
    resource_ref: Option<String>,
    workspace_ref: Option<String>,
    module_ref: Option<String>,
    provider_ref: Option<String>,
    state_ref: Option<String>,
    policy_ref: Option<String>,
    secret_ref: Option<String>,
    connector_ref: Option<String>,
    query_ref: Option<String>,
    rule_ref: Option<String>,
    retention_ref: Option<String>,
    asset_ref: Option<String>,
    scope_ref: Option<String>,
    test_case_ref: Option<String>,
    authorization_ref: Option<String>,
    stop_condition_ref: Option<String>,
    report_ref: Option<String>,
    owner_ref: Option<String>,
    provenance_ref: Option<String>,
    evidence_ref: Option<String>,
}

impl RecordWire {
    fn gcp_iam_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.project_ref.as_deref(),
            self.principal_ref.as_deref(),
            self.binding_ref.as_deref(),
            self.role_ref.as_deref(),
            self.key_ref.as_deref(),
            self.scope_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.provenance_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn kubernetes_rbac_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.cluster_ref.as_deref(),
            self.namespace_ref.as_deref(),
            self.role_ref.as_deref(),
            self.binding_ref.as_deref(),
            self.subject_ref.as_deref(),
            self.verb_ref.as_deref(),
            self.resource_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn terraform_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.workspace_ref.as_deref(),
            self.module_ref.as_deref(),
            self.provider_ref.as_deref(),
            self.state_ref.as_deref(),
            self.policy_ref.as_deref(),
            self.secret_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.provenance_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn cloud_siem_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.workspace_ref.as_deref(),
            self.connector_ref.as_deref(),
            self.query_ref.as_deref(),
            self.rule_ref.as_deref(),
            self.retention_ref.as_deref(),
            self.scope_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.provenance_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn cloud_pentest_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.asset_ref.as_deref(),
            self.scope_ref.as_deref(),
            self.test_case_ref.as_deref(),
            self.authorization_ref.as_deref(),
            self.stop_condition_ref.as_deref(),
            self.report_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.provenance_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn schema(&self) -> Option<(&'static str, [Option<&str>; 10])> {
        match self.kind.as_str() {
            "gcp-iam-permission-audit" => Some((GCP_IAM_SKILL, self.gcp_iam_refs())),
            "kubernetes-rbac-audit" => Some((KUBERNETES_RBAC_SKILL, self.kubernetes_rbac_refs())),
            "terraform-security-audit" => Some((TERRAFORM_SKILL, self.terraform_refs())),
            "cloud-siem-build" => Some((CLOUD_SIEM_SKILL, self.cloud_siem_refs())),
            "cloud-penetration-assessment" => {
                Some((CLOUD_PENTEST_SKILL, self.cloud_pentest_refs()))
            }
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
