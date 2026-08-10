//! Typed boundary for the CP09 cloud-security B08 manifest.
//!
//! BOUNDARY-INVARIANT: this decoder accepts only supplied offline references
//! for DLP, posture-management, CloudTrail, WAF, and workload-protection
//! records. It never connects to a provider, account, endpoint, scanner,
//! runtime, network, or production authority.
// NEGATIVE-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b08.rs
// ROUNDTRIP-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b08.rs

use std::collections::BTreeSet;

use serde::Deserialize;

const DLP_SKILL: &str = "implementing-cloud-dlp-for-data-protection";
const CSPM_SKILL: &str = "implementing-cloud-security-posture-management";
const CLOUDTRAIL_SKILL: &str = "implementing-cloud-trail-log-analysis";
const WAF_SKILL: &str = "implementing-cloud-waf-rules";
const WORKLOAD_SKILL: &str = "implementing-cloud-workload-protection";

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
    resource_ref: Option<String>,
    data_boundary_ref: Option<String>,
    classification_ref: Option<String>,
    policy_ref: Option<String>,
    retention_ref: Option<String>,
    owner_ref: Option<String>,
    evidence_ref: Option<String>,
    scope_ref: Option<String>,
    asset_ref: Option<String>,
    posture_ref: Option<String>,
    control_ref: Option<String>,
    finding_ref: Option<String>,
    severity_ref: Option<String>,
    region_ref: Option<String>,
    trail_ref: Option<String>,
    log_ref: Option<String>,
    event_ref: Option<String>,
    query_ref: Option<String>,
    time_range_ref: Option<String>,
    gateway_ref: Option<String>,
    web_acl_ref: Option<String>,
    rule_ref: Option<String>,
    match_ref: Option<String>,
    action_ref: Option<String>,
    workload_ref: Option<String>,
    image_ref: Option<String>,
    runtime_ref: Option<String>,
}

impl RecordWire {
    fn dlp_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.tenant_ref.as_deref(),
            self.account_ref.as_deref(),
            self.resource_ref.as_deref(),
            self.data_boundary_ref.as_deref(),
            self.classification_ref.as_deref(),
            self.policy_ref.as_deref(),
            self.retention_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn cspm_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.account_ref.as_deref(),
            self.scope_ref.as_deref(),
            self.asset_ref.as_deref(),
            self.posture_ref.as_deref(),
            self.control_ref.as_deref(),
            self.finding_ref.as_deref(),
            self.severity_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn cloudtrail_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.account_ref.as_deref(),
            self.region_ref.as_deref(),
            self.trail_ref.as_deref(),
            self.log_ref.as_deref(),
            self.event_ref.as_deref(),
            self.query_ref.as_deref(),
            self.time_range_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn waf_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.account_ref.as_deref(),
            self.gateway_ref.as_deref(),
            self.web_acl_ref.as_deref(),
            self.rule_ref.as_deref(),
            self.match_ref.as_deref(),
            self.action_ref.as_deref(),
            self.scope_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn workload_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.account_ref.as_deref(),
            self.workload_ref.as_deref(),
            self.image_ref.as_deref(),
            self.runtime_ref.as_deref(),
            self.policy_ref.as_deref(),
            self.finding_ref.as_deref(),
            self.severity_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn schema(&self) -> Option<(&'static str, [Option<&str>; 10])> {
        match self.kind.as_str() {
            "cloud-dlp-data-protection" => Some((DLP_SKILL, self.dlp_refs())),
            "cloud-security-posture-management" => Some((CSPM_SKILL, self.cspm_refs())),
            "cloudtrail-log-analysis" => Some((CLOUDTRAIL_SKILL, self.cloudtrail_refs())),
            "cloud-waf-rules" => Some((WAF_SKILL, self.waf_refs())),
            "cloud-workload-protection" => Some((WORKLOAD_SKILL, self.workload_refs())),
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
