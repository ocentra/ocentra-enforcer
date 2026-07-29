//! The full security-family registry: every one of the 22 `SEC-*` rule
//! ids paired with its constructed [`Validator`]. This is the single
//! source `tests/completeness.rs` walks to prove count-parity against
//! `rules/rules.json`'s `family == "security"` count (22) — one entry per
//! rule id, no orphans, no duplicates.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_validator::validator::Validator;

use super::generic_scanner;
use super::secret_scan::{InlineSecretsValidator, SensitiveFilesValidator};
use crate::boundary::spec::SpecValidator;

/// One registry row: the rule id (as the literal from its owning spec/
/// validator) paired with the constructed [`Validator`] trait object.
pub struct RegistryRow {
    /// The constructed validator for this rule.
    pub validator: Box<dyn Validator>,
}

impl std::fmt::Debug for RegistryRow {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RegistryRow")
            .field("rule_id", self.validator.rule_id())
            .finish()
    }
}

impl RegistryRow {
    pub(crate) fn from_validator(validator: Box<dyn Validator>) -> Self {
        Self { validator }
    }

    /// The canonical rule identity owned by this row's validator.
    pub fn rule_id(&self) -> &enforcer_domain::ids::RuleId {
        self.validator.rule_id()
    }
}

/// Build every one of the 22 security-family rows. Fails closed
/// (propagates the first construction error) rather than silently
/// dropping a malformed entry — a registry that failed to build
/// completely must not be treated as "loaded".
pub fn build_all() -> Result<Vec<RegistryRow>, DecodeError> {
    let mut rows = Vec::new();

    rows.push(RegistryRow::from_validator(Box::new(
        InlineSecretsValidator::new()?,
    )));
    rows.push(RegistryRow::from_validator(Box::new(
        SensitiveFilesValidator::new()?,
    )));

    for spec in generic_scanner::specs()? {
        rows.push(RegistryRow::from_validator(Box::new(SpecValidator::new(
            spec,
        )?)));
    }

    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::build_all;

    #[test]
    fn registry_builds_cleanly() -> Result<(), Box<dyn std::error::Error>> {
        let rows = build_all()?;
        assert_eq!(rows.len(), 22);
        Ok(())
    }
}
