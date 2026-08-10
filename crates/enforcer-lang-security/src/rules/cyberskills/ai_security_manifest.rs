//! `CYBER-AI-MANIFEST.1` — CP09's reusable offline AI-security capability.
//!
//! The validator delegates JSON decoding to the crate boundary and performs
//! only deterministic supplied-manifest validation. It does not invoke a
//! model, connect to an MCP server, send prompts, run a red-team engine, or
//! claim a security outcome. The B01 and B02 packet shapes share one
//! capability rather than creating duplicated validators.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::boundary::ai_security_manifest_wire::is_valid;

const RULE_ID: &str = "CYBER-AI-MANIFEST.1";

/// Native validator for a typed, supplied AI-security manifest.
#[derive(Debug)]
pub struct AiSecurityManifestValidator {
    rule_id: RuleId,
}

impl AiSecurityManifestValidator {
    /// Construct the deterministic AI-security manifest validator.
    pub fn new() -> Result<Self, DecodeError> {
        // ALLOC-JUSTIFICATION: the static rule identity is branded once at
        // construction and then borrowed by every emitted finding.
        let rule_id = RuleId::try_from(RULE_ID.to_owned())?;
        Ok(Self { rule_id })
    }
}

impl Validator for AiSecurityManifestValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        if is_valid(input.source.as_str()) {
            return Vec::new();
        }
        crate::boundary::finding::from_source(
            (&self.rule_id, Severity::Error),
            "AI-security manifest predicate failed",
            "The supplied manifest is malformed or missing a required typed field for its declared AI-security record kind. This is a static schema finding only; no live model, tool, prompt, or security outcome was evaluated.",
            input.file,
            (1, input.source.as_str().lines().next()),
        )
        .into_iter()
        .collect()
    }
}
