//! Typed boundary for the CP09 cloud-security B10 manifest.
//!
//! BOUNDARY-INVARIANT: this decoder accepts only supplied offline references
//! for cloud identity, account enumeration, privilege assessment, asset
//! inventory, and CloudTrail evidence. It never connects to a provider,
//! account, endpoint, scanner, runtime, network, or production authority.
// NEGATIVE-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b10.rs
// ROUNDTRIP-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b10.rs

use std::collections::BTreeSet;

use serde::Deserialize;

const OKTA_SKILL: &str = "managing-cloud-identity-with-okta";
const ACCOUNT_ENUMERATION_SKILL: &str = "performing-aws-account-enumeration-with-scout-suite";
const PRIVILEGE_ASSESSMENT_SKILL: &str = "performing-aws-privilege-escalation-assessment";
const ASSET_INVENTORY_SKILL: &str = "performing-cloud-asset-inventory-with-cartography";
const CLOUDTRAIL_SKILL: &str = "performing-cloud-forensics-with-aws-cloudtrail";

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
    identity_ref: Option<String>,
    group_ref: Option<String>,
    role_ref: Option<String>,
    policy_ref: Option<String>,
    factor_ref: Option<String>,
    enrollment_ref: Option<String>,
    account_ref: Option<String>,
    organization_ref: Option<String>,
    region_ref: Option<String>,
    scope_ref: Option<String>,
    resource_ref: Option<String>,
    permission_ref: Option<String>,
    snapshot_ref: Option<String>,
    principal_ref: Option<String>,
    path_ref: Option<String>,
    finding_ref: Option<String>,
    relationship_ref: Option<String>,
    graph_ref: Option<String>,
    service_ref: Option<String>,
    trail_ref: Option<String>,
    event_ref: Option<String>,
    actor_ref: Option<String>,
    timestamp_ref: Option<String>,
    owner_ref: Option<String>,
    evidence_ref: Option<String>,
}

impl RecordWire {
    fn okta_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.tenant_ref.as_deref(),
            self.identity_ref.as_deref(),
            self.group_ref.as_deref(),
            self.role_ref.as_deref(),
            self.policy_ref.as_deref(),
            self.factor_ref.as_deref(),
            self.enrollment_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn account_enumeration_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.account_ref.as_deref(),
            self.organization_ref.as_deref(),
            self.region_ref.as_deref(),
            self.scope_ref.as_deref(),
            self.resource_ref.as_deref(),
            self.permission_ref.as_deref(),
            self.snapshot_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn privilege_assessment_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.account_ref.as_deref(),
            self.principal_ref.as_deref(),
            self.role_ref.as_deref(),
            self.policy_ref.as_deref(),
            self.permission_ref.as_deref(),
            self.path_ref.as_deref(),
            self.finding_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn asset_inventory_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.tenant_ref.as_deref(),
            self.account_ref.as_deref(),
            self.resource_ref.as_deref(),
            self.relationship_ref.as_deref(),
            self.graph_ref.as_deref(),
            self.service_ref.as_deref(),
            self.region_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn cloudtrail_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.account_ref.as_deref(),
            self.trail_ref.as_deref(),
            self.event_ref.as_deref(),
            self.actor_ref.as_deref(),
            self.timestamp_ref.as_deref(),
            self.region_ref.as_deref(),
            self.resource_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn schema(&self) -> Option<(&'static str, [Option<&str>; 10])> {
        match self.kind.as_str() {
            "okta-cloud-identity" => Some((OKTA_SKILL, self.okta_refs())),
            "aws-account-enumeration" => {
                Some((ACCOUNT_ENUMERATION_SKILL, self.account_enumeration_refs()))
            }
            "aws-privilege-escalation-assessment" => {
                Some((PRIVILEGE_ASSESSMENT_SKILL, self.privilege_assessment_refs()))
            }
            "cloud-asset-inventory" => Some((ASSET_INVENTORY_SKILL, self.asset_inventory_refs())),
            "cloudtrail-forensics" => Some((CLOUDTRAIL_SKILL, self.cloudtrail_refs())),
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
        && manifest.scope == "scope:offline-authorized-static-only"
        && valid_evidence(&manifest.evidence)
        && valid_records(&manifest.records)
}
