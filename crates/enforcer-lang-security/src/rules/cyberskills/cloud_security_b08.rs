//! CYBER-CLOUD-MANIFEST.8 - CP09 supplied-artifact cloud capability.
//!
//! BOUNDARY-INVARIANT: the validator checks only caller-supplied offline JSON
//! for DLP, posture-management, CloudTrail, WAF, and workload references.
//! Provider APIs, accounts, credentials, endpoints, runtimes, scanners,
//! networks, and production outcomes remain outside it.
// NEGATIVE-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b08.rs
// ROUNDTRIP-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b08.rs

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::boundary::cloud_security_b08_manifest_wire::is_valid;

const RULE_ID: &str = "CYBER-CLOUD-MANIFEST.8";

/// Native validator for supplied cloud control and workload manifests.
#[derive(Debug)]
pub struct CloudSecurityManifestB08Validator {
    rule_id: RuleId,
}

impl CloudSecurityManifestB08Validator {
    /// Construct the deterministic B08 cloud-manifest validator.
    pub fn new() -> Result<Self, DecodeError> {
        // ALLOC-JUSTIFICATION: the static rule identity is branded once at
        // construction and borrowed by every emitted finding.
        Ok(Self {
            rule_id: RuleId::try_from(RULE_ID.to_owned())?,
        })
    }
}

impl Validator for CloudSecurityManifestB08Validator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        if is_valid(input.source.as_str()) {
            return Vec::new();
        }
        crate::boundary::finding::from_source(
            (&self.rule_id, Severity::Error),
            "Cloud-security B08 manifest predicate failed",
            "The supplied manifest is malformed or missing a required typed DLP, posture-management, CloudTrail, WAF, or workload reference. This is a static schema finding only; no provider, account, credential, endpoint, scanner, network, runtime, or security outcome was evaluated.",
            input.file,
            (1, input.source.as_str().lines().next()),
        )
        .into_iter()
        .collect()
    }
}
