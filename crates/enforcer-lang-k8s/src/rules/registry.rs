//! Canonical registry for all built-in Kubernetes validators.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_validator::validator::Validator;

use super::spec::SPECS;

/// One constructed built-in Kubernetes validator row.
pub struct RegistryRow {
    /// The constructed validator for this rule.
    pub validator: Box<dyn Validator>,
}

impl std::fmt::Debug for RegistryRow {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RegistryRow")
            .field("rule_id", &self.validator.rule_id())
            .finish_non_exhaustive()
    }
}

/// Build all ten built-in Kubernetes validators or fail the complete registry.
pub fn build_all() -> Result<Vec<RegistryRow>, DecodeError> {
    SPECS
        .iter()
        .map(|spec| {
            let validator = spec.build()?;
            Ok(RegistryRow {
                validator: Box::new(validator),
            })
        })
        .collect()
}
