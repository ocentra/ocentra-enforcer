//! CYBER-CLOUD-MANIFEST.6 - CP09 supplied-artifact cloud capability.
//!
//! BOUNDARY-INVARIANT: the validator checks only caller-supplied offline JSON
//! for consent, attack-emulation, cloud-enumeration, AWS assessment, and AWS
//! Config references. Provider APIs, accounts, credentials, endpoints,
//! runtimes, scanners, networks, and production outcomes remain outside it.
// NEGATIVE-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b06.rs
// ROUNDTRIP-TEST: crates/enforcer-lang-security/tests/cyberskills_cloud_security_manifest_b06.rs

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::boundary::cloud_security_b06_manifest_wire::is_valid;

const RULE_ID: &str = "CYBER-CLOUD-MANIFEST.6";

/// Native validator for supplied cloud authorization and control manifests.
#[derive(Debug)]
pub struct CloudSecurityManifestB06Validator {
    rule_id: RuleId,
}

impl CloudSecurityManifestB06Validator {
    /// Construct the deterministic B06 cloud-manifest validator.
    pub fn new() -> Result<Self, DecodeError> {
        // ALLOC-JUSTIFICATION: the static rule identity is branded once at
        // construction and borrowed by every emitted finding.
        Ok(Self {
            rule_id: RuleId::try_from(RULE_ID.to_owned())?,
        })
    }
}

impl Validator for CloudSecurityManifestB06Validator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        if is_valid(input.source.as_str()) {
            return Vec::new();
        }
        crate::boundary::finding::from_source(
            (&self.rule_id, Severity::Error),
            "Cloud-security B06 manifest predicate failed",
            "The supplied manifest is malformed or missing a required typed consent, authorization, enumeration, AWS assessment, or Config reference. This is a static schema finding only; no provider, account, credential, endpoint, scanner, network, runtime, or security outcome was evaluated.",
            input.file,
            (1, input.source.as_str().lines().next()),
        )
        .into_iter()
        .collect()
    }
}
