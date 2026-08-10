//! `CYBER-API-MANIFEST.1` — CP09's reusable offline API-security capability.
//!
//! The validator accepts only deterministic supplied JSON describing routes,
//! requests, responses, schemas, authorization references, and evidence. It
//! never calls an endpoint, executes a payload, runs Burp/ZAP/fuzzers, or
//! claims a live API security outcome.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::boundary::api_security_manifest_wire::is_valid;

const RULE_ID: &str = "CYBER-API-MANIFEST.1";

/// Native validator for a typed, supplied API-security manifest.
#[derive(Debug)]
pub struct ApiSecurityManifestValidator {
    rule_id: RuleId,
}

impl ApiSecurityManifestValidator {
    /// Construct the deterministic API-security manifest validator.
    pub fn new() -> Result<Self, DecodeError> {
        // ALLOC-JUSTIFICATION: the static rule identity is branded once at
        // construction and then borrowed by every emitted finding.
        let rule_id = RuleId::try_from(RULE_ID.to_owned())?;
        Ok(Self { rule_id })
    }
}

impl Validator for ApiSecurityManifestValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        if is_valid(input.source.as_str()) {
            return Vec::new();
        }
        crate::boundary::finding::from_source(
            (&self.rule_id, Severity::Error),
            "API-security manifest predicate failed",
            "The supplied manifest is malformed or missing a required typed field for its declared API-security record kind. This is a static schema finding only; no live endpoint, payload, scanner, or security outcome was evaluated.",
            input.file,
            (1, input.source.as_str().lines().next()),
        )
        .into_iter()
        .collect()
    }
}
