//! Typed boundary for the CP09 cloud-security B06 manifest.
//!
//! BOUNDARY-INVARIANT: this decoder accepts only supplied offline references
//! for consent, attack-emulation, cloud-enumeration, AWS assessment, and
//! AWS Config records. It never connects to a provider, account, endpoint,
//! scanner, runtime, network, or production authority.
// NEGATIVE-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b06.rs
// ROUNDTRIP-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b06.rs

use std::collections::BTreeSet;

use serde::Deserialize;

const OAUTH_CONSENT_SKILL: &str = "detecting-suspicious-oauth-application-consent";
const STRATUS_SKILL: &str = "emulating-cloud-attacks-with-stratus-red-team";
const CLOUDFOX_SKILL: &str = "enumerating-cloud-with-cloudfox";
const PACU_SKILL: &str = "exploiting-aws-with-pacu";
const CONFIG_SKILL: &str = "implementing-aws-config-rules-for-compliance";

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
    app_ref: Option<String>,
    client_ref: Option<String>,
    publisher_ref: Option<String>,
    scopes_ref: Option<String>,
    consent_ref: Option<String>,
    user_ref: Option<String>,
    policy_ref: Option<String>,
    scenario_ref: Option<String>,
    authorization_ref: Option<String>,
    target_ref: Option<String>,
    phase_ref: Option<String>,
    control_ref: Option<String>,
    stop_ref: Option<String>,
    owner_ref: Option<String>,
    scope_ref: Option<String>,
    inventory_ref: Option<String>,
    query_ref: Option<String>,
    privacy_ref: Option<String>,
    boundary_ref: Option<String>,
    safety_ref: Option<String>,
    config_rule_ref: Option<String>,
    compliance_ref: Option<String>,
    exception_ref: Option<String>,
    review_ref: Option<String>,
    region_ref: Option<String>,
    evidence_ref: Option<String>,
}

impl RecordWire {
    fn oauth_consent_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.tenant_ref.as_deref(),
            self.app_ref.as_deref(),
            self.client_ref.as_deref(),
            self.publisher_ref.as_deref(),
            self.scopes_ref.as_deref(),
            self.consent_ref.as_deref(),
            self.user_ref.as_deref(),
            self.policy_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn stratus_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.tenant_ref.as_deref(),
            self.scenario_ref.as_deref(),
            self.authorization_ref.as_deref(),
            self.target_ref.as_deref(),
            self.phase_ref.as_deref(),
            self.control_ref.as_deref(),
            self.stop_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn cloudfox_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.tenant_ref.as_deref(),
            self.account_ref.as_deref(),
            self.scope_ref.as_deref(),
            self.inventory_ref.as_deref(),
            self.query_ref.as_deref(),
            self.authorization_ref.as_deref(),
            self.privacy_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn pacu_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.tenant_ref.as_deref(),
            self.account_ref.as_deref(),
            self.scenario_ref.as_deref(),
            self.authorization_ref.as_deref(),
            self.boundary_ref.as_deref(),
            self.stop_ref.as_deref(),
            self.safety_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn config_refs(&self) -> [Option<&str>; 10] {
        [
            self.skill_id.as_deref(),
            self.account_ref.as_deref(),
            self.config_rule_ref.as_deref(),
            self.compliance_ref.as_deref(),
            self.scope_ref.as_deref(),
            self.owner_ref.as_deref(),
            self.evidence_ref.as_deref(),
            self.exception_ref.as_deref(),
            self.review_ref.as_deref(),
            self.region_ref.as_deref(),
        ]
    }

    fn schema(&self) -> Option<(&'static str, [Option<&str>; 10])> {
        match self.kind.as_str() {
            "oauth-application-consent" => Some((OAUTH_CONSENT_SKILL, self.oauth_consent_refs())),
            "cloud-attack-emulation" => Some((STRATUS_SKILL, self.stratus_refs())),
            "cloud-asset-enumeration" => Some((CLOUDFOX_SKILL, self.cloudfox_refs())),
            "aws-exploitation-assessment" => Some((PACU_SKILL, self.pacu_refs())),
            "aws-config-compliance" => Some((CONFIG_SKILL, self.config_refs())),
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
