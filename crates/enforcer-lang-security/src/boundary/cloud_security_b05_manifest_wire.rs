//! Typed boundary for the CP09 cloud-security B05 manifest.
//!
//! BOUNDARY-INVARIANT: this decoder accepts only supplied offline references
//! for Azure storage, OAuth token-anomaly, S3 exfiltration-event, serverless
//! injection, and shadow-IT records. It never connects to a provider, log
//! service, scanner, endpoint, tenant, network, workload, or runtime.
// NEGATIVE-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b05.rs
// ROUNDTRIP-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b05.rs

use std::collections::BTreeSet;

use serde::Deserialize;

const AZURE_STORAGE_SKILL: &str = "detecting-misconfigured-azure-storage";
const OAUTH_SKILL: &str = "detecting-oauth-token-theft";
const S3_SKILL: &str = "detecting-s3-data-exfiltration-attempts";
const SERVERLESS_SKILL: &str = "detecting-serverless-function-injection";
const SHADOW_IT_SKILL: &str = "detecting-shadow-it-cloud-usage";

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
    storage_account_ref: Option<String>,
    access_ref: Option<String>,
    network_ref: Option<String>,
    encryption_ref: Option<String>,
    logging_ref: Option<String>,
    policy_ref: Option<String>,
    owner_ref: Option<String>,
    evidence_ref: Option<String>,
    issuer_ref: Option<String>,
    audience_ref: Option<String>,
    client_ref: Option<String>,
    source_ref: Option<String>,
    event_ref: Option<String>,
    replay_ref: Option<String>,
    expiry_ref: Option<String>,
    account_ref: Option<String>,
    bucket_ref: Option<String>,
    principal_ref: Option<String>,
    volume_ref: Option<String>,
    cross_account_ref: Option<String>,
    region_ref: Option<String>,
    function_ref: Option<String>,
    dependency_ref: Option<String>,
    input_ref: Option<String>,
    route_ref: Option<String>,
    identity_ref: Option<String>,
    configuration_ref: Option<String>,
    log_ref: Option<String>,
    scope_ref: Option<String>,
    service_ref: Option<String>,
    data_boundary_ref: Option<String>,
    usage_ref: Option<String>,
    finding_ref: Option<String>,
}

impl RecordWire {
    fn azure_storage_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.tenant_ref.as_deref(),
            self.storage_account_ref.as_deref(),
            self.access_ref.as_deref(),
            self.network_ref.as_deref(),
            self.encryption_ref.as_deref(),
            self.logging_ref.as_deref(),
            self.policy_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn oauth_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.issuer_ref.as_deref(),
            self.audience_ref.as_deref(),
            self.client_ref.as_deref(),
            self.source_ref.as_deref(),
            self.event_ref.as_deref(),
            self.replay_ref.as_deref(),
            self.expiry_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn s3_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.account_ref.as_deref(),
            self.bucket_ref.as_deref(),
            self.principal_ref.as_deref(),
            self.source_ref.as_deref(),
            self.volume_ref.as_deref(),
            self.cross_account_ref.as_deref(),
            self.region_ref.as_deref(),
            self.event_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn serverless_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.function_ref.as_deref(),
            self.dependency_ref.as_deref(),
            self.input_ref.as_deref(),
            self.route_ref.as_deref(),
            self.identity_ref.as_deref(),
            self.configuration_ref.as_deref(),
            self.log_ref.as_deref(),
            self.scope_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn shadow_it_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.tenant_ref.as_deref(),
            self.service_ref.as_deref(),
            self.account_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.data_boundary_ref.as_deref(),
            self.policy_ref.as_deref(),
            self.usage_ref.as_deref(),
            self.finding_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn schema(&self) -> Option<(&'static str, [Option<&str>; 10])> {
        match self.kind.as_str() {
            "azure-storage-misconfiguration" => {
                Some((AZURE_STORAGE_SKILL, self.azure_storage_refs()))
            }
            "oauth-token-anomaly" => Some((OAUTH_SKILL, self.oauth_refs())),
            "s3-exfiltration-event" => Some((S3_SKILL, self.s3_refs())),
            "serverless-function-injection" => Some((SERVERLESS_SKILL, self.serverless_refs())),
            "shadow-it-cloud-usage" => Some((SHADOW_IT_SKILL, self.shadow_it_refs())),
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
