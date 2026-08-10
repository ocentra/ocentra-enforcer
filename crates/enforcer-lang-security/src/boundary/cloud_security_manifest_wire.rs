//! Typed boundary for the CP09 cloud-security B01 manifest.
//!
//! BOUNDARY-INVARIANT: this decoder accepts only supplied offline JSON
//! references for cloud storage, tenant audit, IAM, and benchmark controls.
//! It never connects to a provider, tenant, API, scanner, SIEM, endpoint, or
//! production environment.
// NEGATIVE-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b01.rs
// ROUNDTRIP-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b01.rs

use std::collections::BTreeSet;

use serde::Deserialize;

const CLOUD_STORAGE_SKILL: &str = "analyzing-cloud-storage-access-patterns";
const OFFICE365_SKILL: &str = "analyzing-office365-audit-logs-for-compromise";
const AWS_S3_SKILL: &str = "auditing-aws-s3-bucket-permissions";
const AZURE_AD_SKILL: &str = "auditing-azure-active-directory-configuration";
const CIS_CLOUD_SKILL: &str = "auditing-cloud-with-cis-benchmarks";

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
    asset_ref: Option<String>,
    identity_ref: Option<String>,
    access_ref: Option<String>,
    timestamp_ref: Option<String>,
    region_ref: Option<String>,
    policy_ref: Option<String>,
    owner_ref: Option<String>,
    provenance_ref: Option<String>,
    evidence_ref: Option<String>,
    tenant_ref: Option<String>,
    event_ref: Option<String>,
    source_ref: Option<String>,
    scope_ref: Option<String>,
    bucket_ref: Option<String>,
    public_access_ref: Option<String>,
    encryption_ref: Option<String>,
    versioning_ref: Option<String>,
    role_ref: Option<String>,
    application_ref: Option<String>,
    conditional_access_ref: Option<String>,
    guest_ref: Option<String>,
    credential_ref: Option<String>,
    control_ref: Option<String>,
    resource_ref: Option<String>,
    config_ref: Option<String>,
    benchmark_ref: Option<String>,
    exception_ref: Option<String>,
}

impl RecordWire {
    fn cloud_storage_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.asset_ref.as_deref(),
            self.identity_ref.as_deref(),
            self.access_ref.as_deref(),
            self.timestamp_ref.as_deref(),
            self.region_ref.as_deref(),
            self.policy_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.provenance_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn office365_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.tenant_ref.as_deref(),
            self.event_ref.as_deref(),
            self.identity_ref.as_deref(),
            self.source_ref.as_deref(),
            self.timestamp_ref.as_deref(),
            self.scope_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.provenance_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn aws_s3_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.bucket_ref.as_deref(),
            self.policy_ref.as_deref(),
            self.public_access_ref.as_deref(),
            self.encryption_ref.as_deref(),
            self.versioning_ref.as_deref(),
            self.region_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.provenance_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn azure_directory_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.tenant_ref.as_deref(),
            self.role_ref.as_deref(),
            self.application_ref.as_deref(),
            self.conditional_access_ref.as_deref(),
            self.guest_ref.as_deref(),
            self.credential_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.provenance_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn cis_cloud_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.control_ref.as_deref(),
            self.resource_ref.as_deref(),
            self.config_ref.as_deref(),
            self.benchmark_ref.as_deref(),
            self.scope_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.provenance_ref.as_deref(),
            self.exception_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn schema(&self) -> Option<(&'static str, [Option<&str>; 10])> {
        match self.kind.as_str() {
            "cloud-storage-access-patterns" => {
                Some((CLOUD_STORAGE_SKILL, self.cloud_storage_refs()))
            }
            "office365-audit-log-review" => Some((OFFICE365_SKILL, self.office365_refs())),
            "aws-s3-permission-audit" => Some((AWS_S3_SKILL, self.aws_s3_refs())),
            "azure-directory-configuration-audit" => {
                Some((AZURE_AD_SKILL, self.azure_directory_refs()))
            }
            "cis-cloud-benchmark-review" => Some((CIS_CLOUD_SKILL, self.cis_cloud_refs())),
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
