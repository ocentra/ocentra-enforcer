//! Typed boundary for the CP09 cloud-security B12 manifest.
//!
//! BOUNDARY-INVARIANT: this decoder accepts only caller-supplied offline
//! references for GCP bucket testing, GCP assessment, serverless review, S3
//! remediation planning, and AWS WAF gateway records. It never connects to a
//! provider, account, function, bucket, gateway, scanner, runtime, network,
//! or production authority.
// NEGATIVE-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b12.rs
// ROUNDTRIP-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b12.rs

use std::collections::BTreeSet;

use serde::Deserialize;

const GCP_BUCKETBRUTE_SKILL: &str = "performing-gcp-penetration-testing-with-gcpbucketbrute";
const FORSETI_SKILL: &str = "performing-gcp-security-assessment-with-forseti";
const SERVERLESS_REVIEW_SKILL: &str = "performing-serverless-function-security-review";
const S3_REMEDIATION_SKILL: &str = "remediating-s3-bucket-misconfiguration";
const AWS_WAF_SKILL: &str = "securing-api-gateway-with-aws-waf";

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
    refs: Vec<String>,
}

impl RecordWire {
    fn schema(&self) -> Option<(&'static str, usize)> {
        match self.kind.as_str() {
            "gcp-bucketbrute-assessment" => Some((GCP_BUCKETBRUTE_SKILL, 9)),
            "gcp-forseti-security-assessment" => Some((FORSETI_SKILL, 9)),
            "serverless-function-security-review" => Some((SERVERLESS_REVIEW_SKILL, 9)),
            "s3-remediation-plan" => Some((S3_REMEDIATION_SKILL, 9)),
            "aws-waf-gateway-security" => Some((AWS_WAF_SKILL, 9)),
            _ => None,
        }
    }

    fn is_valid(&self) -> bool {
        let Some((expected_skill, expected_refs)) = self.schema() else {
            return false;
        };
        self.skill_id.as_deref() == Some(expected_skill)
            && self.refs.len() == expected_refs
            && self.refs.iter().all(|value| valid_ref(value))
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
        && manifest.scope == "scope:offline-authorized-static-only"
        && valid_evidence(&manifest.evidence)
        && valid_records(&manifest.records)
}
