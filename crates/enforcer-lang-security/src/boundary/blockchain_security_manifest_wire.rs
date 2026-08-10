//! Typed boundary for the CP09 blockchain-security manifest.
//!
//! BOUNDARY-INVARIANT: this decoder accepts only supplied offline JSON
//! references. It never connects to a chain, node, wallet, compiler, RPC
//! provider, deployment, scanner, symbolic executor, or test runner.
// NEGATIVE-TEST: crates/enforcer-lang-security/tests/cyberskills_blockchain_security_manifest_b01.rs
// ROUNDTRIP-TEST: crates/enforcer-lang-security/tests/cyberskills_blockchain_security_manifest_b01.rs

use std::collections::BTreeSet;

use serde::Deserialize;

const ETHEREUM_SKILL: &str = "analyzing-ethereum-smart-contract-vulnerabilities";
const FOUNDRY_SKILL: &str = "auditing-foundry-smart-contract-security";

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
    contract_ref: Option<String>,
    source_ref: Option<String>,
    bytecode_ref: Option<String>,
    abi_ref: Option<String>,
    test_ref: Option<String>,
    invariant_ref: Option<String>,
    dependency_ref: Option<String>,
    chain_ref: Option<String>,
    address_ref: Option<String>,
    owner_ref: Option<String>,
    scope_ref: Option<String>,
    provenance_ref: Option<String>,
    finding_ref: Option<String>,
    evidence_ref: Option<String>,
    key_hygiene_ref: Option<String>,
    deployment_ref: Option<String>,
}

impl RecordWire {
    fn expected_skill(&self) -> Option<&'static str> {
        match self.kind.as_str() {
            "ethereum-contract-audit" => Some(ETHEREUM_SKILL),
            "foundry-project-audit" => Some(FOUNDRY_SKILL),
            _ => None,
        }
    }

    fn required_refs(&self) -> [Option<&str>; 10] {
        match self.kind.as_str() {
            "ethereum-contract-audit" => [
                self.skill_id.as_deref(),
                self.contract_ref.as_deref(),
                self.source_ref.as_deref(),
                self.bytecode_ref.as_deref(),
                self.abi_ref.as_deref(),
                self.chain_ref.as_deref(),
                self.address_ref.as_deref(),
                self.provenance_ref.as_deref(),
                self.finding_ref.as_deref(),
                self.evidence_ref.as_deref(),
            ],
            "foundry-project-audit" => [
                self.skill_id.as_deref(),
                self.contract_ref.as_deref(),
                self.source_ref.as_deref(),
                self.test_ref.as_deref(),
                self.invariant_ref.as_deref(),
                self.dependency_ref.as_deref(),
                self.chain_ref.as_deref(),
                self.owner_ref.as_deref(),
                self.scope_ref.as_deref(),
                self.provenance_ref.as_deref(),
            ],
            _ => [None; 10],
        }
    }

    fn optional_refs(&self) -> [Option<&str>; 3] {
        [
            self.key_hygiene_ref.as_deref(),
            self.deployment_ref.as_deref(),
            self.evidence_ref.as_deref(),
        ]
    }

    fn is_valid(&self) -> bool {
        let Some(expected_skill) = self.expected_skill() else {
            return false;
        };
        let required = self.required_refs();
        let optional = self.optional_refs();
        required.first().and_then(|value| *value) == Some(expected_skill)
            && required
                .iter()
                .skip(1)
                .all(|value| value.is_some_and(valid_ref))
            && optional.iter().filter_map(|value| *value).all(valid_ref)
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

pub(crate) fn is_valid(source: &str) -> bool {
    let Ok(manifest) = serde_json::from_str::<ManifestWire>(source) else {
        return false;
    };
    manifest.schema_version == 1
        && !manifest.bundle_id.trim().is_empty()
        && !manifest.owner.trim().is_empty()
        && !manifest.scope.trim().is_empty()
        && valid_evidence(&manifest.evidence)
        && !manifest.records.is_empty()
        && manifest.records.len() == 2
        && manifest.records.iter().all(RecordWire::is_valid)
}
