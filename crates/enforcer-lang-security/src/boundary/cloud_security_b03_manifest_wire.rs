//! Typed boundary for the CP09 cloud-security B03 manifest.
//!
//! BOUNDARY-INVARIANT: this decoder accepts only supplied offline JSON
//! references for cloud telemetry, secret-review, finding-workflow,
//! escalation-analysis, and Azure movement records. It never connects to a
//! provider, log service, scanner, repository, endpoint, tenant, or network.
// NEGATIVE-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b03.rs
// ROUNDTRIP-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b03.rs

use std::collections::BTreeSet;

use serde::Deserialize;

const CLOUDTRAIL_SKILL: &str = "detecting-aws-cloudtrail-anomalies";
const SECRET_SKILL: &str = "detecting-aws-credential-exposure-with-trufflehog";
const GUARDDUTY_SKILL: &str = "detecting-aws-guardduty-findings-automation";
const IAM_SKILL: &str = "detecting-aws-iam-privilege-escalation";
const AZURE_SKILL: &str = "detecting-azure-lateral-movement";

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
    account_ref: Option<String>,
    event_ref: Option<String>,
    actor_ref: Option<String>,
    resource_ref: Option<String>,
    action_ref: Option<String>,
    anomaly_ref: Option<String>,
    repository_ref: Option<String>,
    artifact_ref: Option<String>,
    secret_ref: Option<String>,
    source_ref: Option<String>,
    scan_request_ref: Option<String>,
    scope_ref: Option<String>,
    owner_ref: Option<String>,
    severity_ref: Option<String>,
    finding_ref: Option<String>,
    rule_ref: Option<String>,
    workflow_ref: Option<String>,
    principal_ref: Option<String>,
    policy_ref: Option<String>,
    trust_ref: Option<String>,
    graph_ref: Option<String>,
    tenant_ref: Option<String>,
    identity_ref: Option<String>,
    host_ref: Option<String>,
    route_ref: Option<String>,
    relationship_ref: Option<String>,
    risk_ref: Option<String>,
    provenance_ref: Option<String>,
    evidence_ref: Option<String>,
}

impl RecordWire {
    fn cloudtrail_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.account_ref.as_deref(),
            self.event_ref.as_deref(),
            self.actor_ref.as_deref(),
            self.resource_ref.as_deref(),
            self.action_ref.as_deref(),
            self.anomaly_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.provenance_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn secret_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.repository_ref.as_deref(),
            self.artifact_ref.as_deref(),
            self.secret_ref.as_deref(),
            self.source_ref.as_deref(),
            self.scan_request_ref.as_deref(),
            self.scope_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.severity_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn guardduty_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.account_ref.as_deref(),
            self.finding_ref.as_deref(),
            self.event_ref.as_deref(),
            self.rule_ref.as_deref(),
            self.severity_ref.as_deref(),
            self.scope_ref.as_deref(),
            self.workflow_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn iam_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.account_ref.as_deref(),
            self.principal_ref.as_deref(),
            self.policy_ref.as_deref(),
            self.action_ref.as_deref(),
            self.trust_ref.as_deref(),
            self.graph_ref.as_deref(),
            self.scope_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn azure_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.tenant_ref.as_deref(),
            self.identity_ref.as_deref(),
            self.host_ref.as_deref(),
            self.event_ref.as_deref(),
            self.route_ref.as_deref(),
            self.relationship_ref.as_deref(),
            self.risk_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn schema(&self) -> Option<(&'static str, [Option<&str>; 10])> {
        match self.kind.as_str() {
            "aws-cloudtrail-anomaly" => Some((CLOUDTRAIL_SKILL, self.cloudtrail_refs())),
            "aws-secret-exposure-review" => Some((SECRET_SKILL, self.secret_refs())),
            "aws-guardduty-finding-automation" => Some((GUARDDUTY_SKILL, self.guardduty_refs())),
            "aws-iam-escalation-analysis" => Some((IAM_SKILL, self.iam_refs())),
            "azure-lateral-movement" => Some((AZURE_SKILL, self.azure_refs())),
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
