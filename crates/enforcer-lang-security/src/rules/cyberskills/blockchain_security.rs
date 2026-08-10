//! `CYBER-BLOCKCHAIN-MANIFEST.1` — CP09's supplied-artifact blockchain
//! security capability.
//!
//! The validator converts the vendor skills' smart-contract and Foundry audit
//! intent into deterministic checks over a caller-supplied JSON manifest. It
//! does not parse Solidity, execute bytecode, run symbolic execution, or
//! connect to any blockchain authority.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::boundary::blockchain_security_manifest_wire::is_valid;

const RULE_ID: &str = "CYBER-BLOCKCHAIN-MANIFEST.1";

/// Native validator for supplied smart-contract and Foundry audit manifests.
#[derive(Debug)]
pub struct BlockchainSecurityManifestValidator {
    rule_id: RuleId,
}

impl BlockchainSecurityManifestValidator {
    /// Construct the deterministic blockchain-security manifest validator.
    pub fn new() -> Result<Self, DecodeError> {
        // ALLOC-JUSTIFICATION: the static rule identity is branded once at
        // construction and borrowed by every emitted finding.
        Ok(Self {
            rule_id: RuleId::try_from(RULE_ID.to_owned())?,
        })
    }
}

impl Validator for BlockchainSecurityManifestValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        if is_valid(input.source.as_str()) {
            return Vec::new();
        }
        crate::boundary::finding::from_source(
            (&self.rule_id, Severity::Error),
            "Blockchain-security manifest predicate failed",
            "The supplied manifest is malformed or missing a required typed smart-contract or Foundry audit reference. This is a static schema finding only; no chain, compiler, execution, wallet, scanner, or security outcome was evaluated.",
            input.file,
            (1, input.source.as_str().lines().next()),
        )
        .into_iter()
        .collect()
    }
}
