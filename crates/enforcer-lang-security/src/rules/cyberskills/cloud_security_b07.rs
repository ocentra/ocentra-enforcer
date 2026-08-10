//! CYBER-CLOUD-MANIFEST.7 - CP09 supplied-artifact cloud capability.
//!
//! BOUNDARY-INVARIANT: the validator checks only caller-supplied offline JSON
//! for data classification, enclave trust, Security Hub, compliance, and
//! Defender references. Provider APIs, accounts, credentials, endpoints,
//! runtimes, scanners, networks, and production outcomes remain outside it.
// NEGATIVE-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b07.rs
// ROUNDTRIP-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b07.rs

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::boundary::cloud_security_b07_manifest_wire::is_valid;

const RULE_ID: &str = "CYBER-CLOUD-MANIFEST.7";

/// Native validator for supplied cloud control and posture manifests.
#[derive(Debug)]
pub struct CloudSecurityManifestB07Validator {
    rule_id: RuleId,
}

impl CloudSecurityManifestB07Validator {
    /// Construct the deterministic B07 cloud-manifest validator.
    pub fn new() -> Result<Self, DecodeError> {
        // ALLOC-JUSTIFICATION: the static rule identity is branded once at
        // construction and borrowed by every emitted finding.
        Ok(Self {
            rule_id: RuleId::try_from(RULE_ID.to_owned())?,
        })
    }
}

impl Validator for CloudSecurityManifestB07Validator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        if is_valid(input.source.as_str()) {
            return Vec::new();
        }
        crate::boundary::finding::from_source(
            (&self.rule_id, Severity::Error),
            "Cloud-security B07 manifest predicate failed",
            "The supplied manifest is malformed or missing a required typed classification, enclave, Security Hub, compliance, or Defender reference. This is a static schema finding only; no provider, account, credential, endpoint, scanner, network, runtime, or security outcome was evaluated.",
            input.file,
            (1, input.source.as_str().lines().next()),
        )
        .into_iter()
        .collect()
    }
}
