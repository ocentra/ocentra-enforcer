//! The full K8S-family registry: every one of the `K8S-*` rule ids paired
//! with its constructed `Validator`. `tests/completeness.rs` walks this to
//! prove no duplicate rule id and a total matching [`super::spec::SPECS`]'s
//! length — the count-parity shape the sibling lang crates use, scoped to
//! this crate's own spec table since `rules/rules.json` carries no
//! `language == "k8s"` rows yet (arc-12 introduces the family).

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::ids::RuleId;
use enforcer_validator::validator::Validator;
use std::fmt;

use super::spec::SPECS;

/// One registry row: the rule id (as the literal from its owning spec)
/// paired with the constructed [`Validator`] trait object.
pub struct RegistryRow {
    /// The rule id this row proves, e.g. `K8S-1.1`.
    pub rule_id: RuleId,
    /// The constructed validator for this rule.
    pub validator: Box<dyn Validator>,
}

impl fmt::Debug for RegistryRow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RegistryRow")
            .field("rule_id", &self.rule_id)
            .field("validator", &"<validator>")
            .finish()
    }
}

/// Build every K8S-family row. Fails closed (propagates the first
/// construction error) rather than silently dropping a malformed entry.
pub fn build_all() -> Result<Vec<RegistryRow>, DecodeError> {
    let mut rows = Vec::with_capacity(SPECS.len());
    for spec in SPECS {
        let validator = spec.build()?;
        rows.push(RegistryRow {
            // CLONE-JUSTIFICATION: the registry exposes a stable owned ID
            // while the validator must retain its own ID for findings.
            rule_id: validator.rule_id().clone(),
            validator: Box::new(validator),
        });
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::build_all;

    #[test]
    fn registry_builds_cleanly() -> Result<(), Box<dyn std::error::Error>> {
        let rows = build_all()?;
        assert!(!rows.is_empty());
        Ok(())
    }
}
