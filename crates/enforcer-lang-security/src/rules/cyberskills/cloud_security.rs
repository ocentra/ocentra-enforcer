//! `CYBER-CLOUD-MANIFEST.1` — CP09's supplied-artifact cloud capability.
//!
//! The validator converts five cloud-security intents into deterministic
//! checks over a caller-supplied JSON manifest. It does not connect to cloud
//! providers, tenants, APIs, scanners, SIEM systems, endpoints, or production.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::boundary::cloud_security_manifest_wire::is_valid;

const RULE_ID: &str = "CYBER-CLOUD-MANIFEST.1";

/// Native validator for supplied cloud-security configuration and evidence manifests.
#[derive(Debug)]
pub struct CloudSecurityManifestValidator {
    rule_id: RuleId,
}

impl CloudSecurityManifestValidator {
    /// Construct the deterministic cloud-security manifest validator.
    pub fn new() -> Result<Self, DecodeError> {
        // ALLOC-JUSTIFICATION: the static rule identity is branded once at
        // construction and borrowed by every emitted finding.
        Ok(Self {
            rule_id: RuleId::try_from(RULE_ID.to_owned())?,
        })
    }
}

impl Validator for CloudSecurityManifestValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        if is_valid(input.source.as_str()) {
            return Vec::new();
        }
        crate::boundary::finding::from_source(
            (&self.rule_id, Severity::Error),
            "Cloud-security manifest predicate failed",
            "The supplied manifest is malformed or missing a required typed cloud-security reference. This is a static schema finding only; no cloud API, tenant, scanner, SIEM, endpoint, provider, or security outcome was evaluated.",
            input.file,
            (1, input.source.as_str().lines().next()),
        )
        .into_iter()
        .collect()
    }
}
