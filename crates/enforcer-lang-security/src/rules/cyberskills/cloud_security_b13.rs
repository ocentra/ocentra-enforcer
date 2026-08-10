//! CYBER-CLOUD-MANIFEST.13 - CP09 supplied-artifact cloud capability.
//!
//! BOUNDARY-INVARIANT: the validator checks only caller-supplied offline JSON
//! for IAM, Lambda, Azure Defender, registry, Kubernetes, and serverless
//! records. Provider APIs, accounts, credentials, clusters, registries,
//! functions, scanners, networks, and production outcomes remain outside it.
// NEGATIVE-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b13.rs
// ROUNDTRIP-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b13.rs

use std::collections::BTreeSet;

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::boundary::cloud_security_b13_manifest_wire::{
    parse, EvidenceWire, ManifestWire, RecordWire,
};

const RULE_ID: &str = "CYBER-CLOUD-MANIFEST.13";
const AWS_IAM_SKILL: &str = "securing-aws-iam-permissions";
const AWS_LAMBDA_SKILL: &str = "securing-aws-lambda-execution-roles";
const AZURE_DEFENDER_SKILL: &str = "securing-azure-with-microsoft-defender";
const REGISTRY_SKILL: &str = "securing-container-registry-images";
const KUBERNETES_SKILL: &str = "securing-kubernetes-on-cloud";
const SERVERLESS_SKILL: &str = "securing-serverless-functions";

#[derive(Clone, Copy)]
struct ExpectedSkill(&'static str);

struct KindValue<'a>(&'a str);

struct ReferenceValue<'a>(&'a str);

struct EvidenceValues<'a>(&'a [EvidenceWire]);

struct SourceValue<'a>(&'a str);

#[derive(Clone, Copy)]
struct Predicate(bool);

fn expected_skill(kind: KindValue<'_>) -> Option<ExpectedSkill> {
    [
        ("aws-iam-permissions", ExpectedSkill(AWS_IAM_SKILL)),
        ("aws-lambda-execution-role", ExpectedSkill(AWS_LAMBDA_SKILL)),
        (
            "azure-defender-control",
            ExpectedSkill(AZURE_DEFENDER_SKILL),
        ),
        ("container-registry-image", ExpectedSkill(REGISTRY_SKILL)),
        (
            "kubernetes-cloud-hardening",
            ExpectedSkill(KUBERNETES_SKILL),
        ),
        (
            "serverless-function-hardening",
            ExpectedSkill(SERVERLESS_SKILL),
        ),
    ]
    .into_iter()
    .find_map(|(candidate, skill)| (candidate == kind.0).then_some(skill))
}

fn valid_ref(value: ReferenceValue<'_>) -> Predicate {
    let Some((kind, identifier)) = value.0.split_once(':') else {
        return Predicate(false);
    };
    Predicate(
        !kind.is_empty()
            && !identifier.is_empty()
            && !value.0.chars().any(char::is_whitespace)
            && !kind.chars().any(char::is_whitespace),
    )
}

fn valid_record(record: &RecordWire) -> Predicate {
    Predicate(
        record.skill_id.as_deref().is_some_and(|skill| {
            expected_skill(KindValue(&record.kind)).is_some_and(|expected| skill == expected.0)
        }) && record.refs.len() == 9
            && record
                .refs
                .iter()
                .all(|value| valid_ref(ReferenceValue(value)).0),
    )
}

fn valid_evidence(evidence: EvidenceValues<'_>) -> Predicate {
    let mut seen = BTreeSet::new();
    Predicate(
        !evidence.0.is_empty()
            && evidence.0.iter().all(|entry| {
                valid_ref(ReferenceValue(&entry.reference)).0
                    && !entry.kind.trim().is_empty()
                    && seen.insert(format!("{}:{}", entry.kind, entry.reference))
            }),
    )
}

fn valid_manifest(manifest: &ManifestWire) -> Predicate {
    let mut kinds = BTreeSet::new();
    Predicate(
        manifest.schema_version == 1
            && !manifest.bundle_id.trim().is_empty()
            && !manifest.owner.trim().is_empty()
            && manifest.scope == "scope:offline-authorized-static-only"
            && valid_evidence(EvidenceValues(&manifest.evidence)).0
            && manifest.records.len() == 6
            && manifest.records.iter().all(|record| {
                // CLONE-JUSTIFICATION: duplicate-kind detection owns a stable
                // key while the borrowed wire record remains parse-owned.
                kinds.insert(record.kind.clone()) && valid_record(record).0
            }),
    )
}

fn manifest_is_valid(source: SourceValue<'_>) -> Predicate {
    Predicate(parse(source.0).is_ok_and(|manifest| valid_manifest(&manifest).0))
}

/// Native validator for supplied B13 cloud-security manifests.
#[derive(Debug)]
pub struct CloudSecurityManifestB13Validator {
    rule_id: RuleId,
}

impl CloudSecurityManifestB13Validator {
    /// Construct the deterministic B13 cloud-manifest validator.
    pub fn new() -> Result<Self, DecodeError> {
        // ALLOC-JUSTIFICATION: the static rule identity is branded once at
        // construction and borrowed by every emitted finding.
        Ok(Self {
            rule_id: RuleId::try_from(RULE_ID.to_owned())?,
        })
    }
}

impl Validator for CloudSecurityManifestB13Validator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        if manifest_is_valid(SourceValue(input.source.as_str())).0 {
            return Vec::new();
        }
        crate::boundary::finding::from_source(
            (&self.rule_id, Severity::Error),
            "Cloud-security B13 manifest predicate failed",
            "The supplied manifest is malformed or missing a required typed IAM, Lambda, Azure Defender, registry, Kubernetes, or serverless reference. This is a static schema finding only; no provider, account, credential, cluster, registry, function, scanner, network, runtime, or security outcome was evaluated.",
            input.file,
            (1, input.source.as_str().lines().next()),
        )
        .into_iter()
        .collect()
    }
}
