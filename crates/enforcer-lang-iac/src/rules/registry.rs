//! Canonical registry for all built-in IaC validators.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::ids::RuleId;
use enforcer_validator::validator::Validator;

use super::cloudformation;
use super::kubernetes;
use super::spec::SpecValidator;
use super::terraform;

/// One fully constructed built-in IaC validator row.
pub struct RegistryRow {
    pub rule_id: RuleId,
    pub validator: Box<dyn Validator>,
}

impl std::fmt::Debug for RegistryRow {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RegistryRow")
            .field("rule_id", &self.rule_id)
            .finish_non_exhaustive()
    }
}

/// Build all eight built-in IaC validators or fail the complete registry.
pub fn build_all() -> Result<Vec<RegistryRow>, DecodeError> {
    terraform::SPECS
        .iter()
        .chain(cloudformation::SPECS)
        .chain(kubernetes::SPECS)
        .map(|spec| {
            let validator = SpecValidator::new(*spec)?;
            Ok(RegistryRow {
                rule_id: spec.rule.id(),
                validator: Box::new(validator),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use enforcer_domain::ids::BuiltInIacRule;

    use super::build_all;

    #[test]
    fn registry_builds_every_canonical_iac_rule() -> Result<(), Box<dyn std::error::Error>> {
        let rows = build_all()?;
        assert_eq!(rows.len(), BuiltInIacRule::ALL.len());
        Ok(())
    }
}
