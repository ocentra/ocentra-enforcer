//! `CYBER-CLOUD-MANIFEST.3` — CP09's supplied-artifact cloud capability.
//!
//! BOUNDARY-INVARIANT: the validator checks only caller-supplied offline JSON
//! for CloudTrail, secret-review, GuardDuty, IAM-analysis, and Azure movement
//! records. Provider APIs, logs, repositories, credentials, endpoints,
//! scanners, networks, and production outcomes remain outside this rule.
// NEGATIVE-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b03.rs
// ROUNDTRIP-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b03.rs

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::boundary::cloud_security_b03_manifest_wire::is_valid;

const RULE_ID: &str = "CYBER-CLOUD-MANIFEST.3";

/// Native validator for supplied cloud telemetry and risk manifests.
#[derive(Debug)]
pub struct CloudSecurityManifestB03Validator {
    rule_id: RuleId,
}

impl CloudSecurityManifestB03Validator {
    /// Construct the deterministic B03 cloud-manifest validator.
    pub fn new() -> Result<Self, DecodeError> {
        // ALLOC-JUSTIFICATION: the static rule identity is branded once at
        // construction and borrowed by every emitted finding.
        Ok(Self {
            rule_id: RuleId::try_from(RULE_ID.to_owned())?,
        })
    }
}

impl Validator for CloudSecurityManifestB03Validator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        if is_valid(input.source.as_str()) {
            return Vec::new();
        }
        crate::boundary::finding::from_source(
            (&self.rule_id, Severity::Error),
            "Cloud-security B03 manifest predicate failed",
            "The supplied manifest is malformed or missing a required typed cloud-telemetry or risk reference. This is a static schema finding only; no provider, log service, repository, credential, endpoint, scanner, network, or security outcome was evaluated.",
            input.file,
            (1, input.source.as_str().lines().next()),
        )
        .into_iter()
        .collect()
    }
}
