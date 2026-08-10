//! Typed boundary for the CP09 cloud-security B11 manifest.
//!
//! BOUNDARY-INVARIANT: this decoder accepts only supplied offline references
//! for zero-trust admission, cloud-log forensics, Falco evidence, Detective
//! investigations, and authorized Pacu assessment records. It never connects
//! to a provider, account, cluster, endpoint, scanner, runtime, network, or
//! production authority.
// NEGATIVE-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b11.rs
// ROUNDTRIP-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b11.rs

use std::collections::BTreeSet;

use serde::Deserialize;

const ZTNA_SKILL: &str = "implementing-zero-trust-network-access";
const CLOUD_LOG_FORENSICS_SKILL: &str = "performing-cloud-log-forensics-with-athena";
const FALCO_FORENSICS_SKILL: &str = "performing-cloud-native-forensics-with-falco";
const DETECTIVE_HUNTING_SKILL: &str = "performing-cloud-native-threat-hunting-with-aws-detective";
const PACU_TESTING_SKILL: &str = "performing-cloud-penetration-testing-with-pacu";

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
    device_ref: Option<String>,
    posture_ref: Option<String>,
    network_ref: Option<String>,
    admission_ref: Option<String>,
    saml_ref: Option<String>,
    mtls_ref: Option<String>,
    dns_policy_ref: Option<String>,
    saas_ref: Option<String>,
    session_ref: Option<String>,
    owner_ref: Option<String>,
    config_ref: Option<String>,
    evidence_ref: Option<String>,
    account_ref: Option<String>,
    log_group_ref: Option<String>,
    query_ref: Option<String>,
    time_window_ref: Option<String>,
    event_ref: Option<String>,
    actor_ref: Option<String>,
    resource_ref: Option<String>,
    cluster_ref: Option<String>,
    node_ref: Option<String>,
    workload_ref: Option<String>,
    rule_ref: Option<String>,
    process_ref: Option<String>,
    container_ref: Option<String>,
    severity_ref: Option<String>,
    graph_ref: Option<String>,
    investigation_ref: Option<String>,
    entity_ref: Option<String>,
    relationship_ref: Option<String>,
    timeline_ref: Option<String>,
    hypothesis_ref: Option<String>,
    scope_ref: Option<String>,
    authorization_ref: Option<String>,
    service_ref: Option<String>,
    identity_ref: Option<String>,
    permission_ref: Option<String>,
    path_ref: Option<String>,
}

impl RecordWire {
    fn ztna_refs(&self) -> Vec<Option<&str>> {
        vec![
            self.skill_id.as_deref(),
            self.device_ref.as_deref(),
            self.posture_ref.as_deref(),
            self.network_ref.as_deref(),
            self.admission_ref.as_deref(),
            self.saml_ref.as_deref(),
            self.mtls_ref.as_deref(),
            self.dns_policy_ref.as_deref(),
            self.saas_ref.as_deref(),
            self.session_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.config_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn cloud_log_forensics_refs(&self) -> Vec<Option<&str>> {
        vec![
            self.skill_id.as_deref(),
            self.account_ref.as_deref(),
            self.log_group_ref.as_deref(),
            self.query_ref.as_deref(),
            self.time_window_ref.as_deref(),
            self.event_ref.as_deref(),
            self.actor_ref.as_deref(),
            self.resource_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn falco_forensics_refs(&self) -> Vec<Option<&str>> {
        vec![
            self.skill_id.as_deref(),
            self.cluster_ref.as_deref(),
            self.node_ref.as_deref(),
            self.workload_ref.as_deref(),
            self.rule_ref.as_deref(),
            self.event_ref.as_deref(),
            self.process_ref.as_deref(),
            self.container_ref.as_deref(),
            self.severity_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn detective_hunting_refs(&self) -> Vec<Option<&str>> {
        vec![
            self.skill_id.as_deref(),
            self.account_ref.as_deref(),
            self.graph_ref.as_deref(),
            self.investigation_ref.as_deref(),
            self.entity_ref.as_deref(),
            self.relationship_ref.as_deref(),
            self.timeline_ref.as_deref(),
            self.hypothesis_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn pacu_testing_refs(&self) -> Vec<Option<&str>> {
        vec![
            self.skill_id.as_deref(),
            self.account_ref.as_deref(),
            self.scope_ref.as_deref(),
            self.authorization_ref.as_deref(),
            self.service_ref.as_deref(),
            self.identity_ref.as_deref(),
            self.permission_ref.as_deref(),
            self.path_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn schema(&self) -> Option<(&'static str, Vec<Option<&str>>)> {
        match self.kind.as_str() {
            "ztna-network-admission" => Some((ZTNA_SKILL, self.ztna_refs())),
            "cloud-log-forensics-athena" => {
                Some((CLOUD_LOG_FORENSICS_SKILL, self.cloud_log_forensics_refs()))
            }
            "cloud-native-forensics-falco" => {
                Some((FALCO_FORENSICS_SKILL, self.falco_forensics_refs()))
            }
            "cloud-native-threat-hunting" => {
                Some((DETECTIVE_HUNTING_SKILL, self.detective_hunting_refs()))
            }
            "cloud-penetration-testing-pacu" => {
                Some((PACU_TESTING_SKILL, self.pacu_testing_refs()))
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
        && manifest.scope == "scope:offline-authorized-static-only"
        && valid_evidence(&manifest.evidence)
        && valid_records(&manifest.records)
}
