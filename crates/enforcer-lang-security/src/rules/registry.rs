//! The full security-family registry: every one of the 22 `SEC-*` rule
//! ids paired with its constructed [`Validator`]. This is the single
//! source `tests/completeness.rs` walks to prove count-parity against
//! `rules/rules.json`'s `family == "security"` count (22) — one entry per
//! rule id, no orphans, no duplicates.

use enforcer_core::error::DecodeError;
use enforcer_validator::validator::Validator;

use super::generic_scanner;
use super::secret_scan::{InlineSecretsValidator, SensitiveFilesValidator};
use super::spec::SpecValidator;

/// One registry row: the rule id (as the literal from its owning spec/
/// validator) paired with the constructed [`Validator`] trait object.
pub struct RegistryRow {
    /// The rule id this row proves, e.g. `SEC-2.1`.
    pub rule_id: &'static str,
    /// The constructed validator for this rule.
    pub validator: Box<dyn Validator>,
}

/// Build every one of the 22 security-family rows. Fails closed
/// (propagates the first construction error) rather than silently
/// dropping a malformed entry — a registry that failed to build
/// completely must not be treated as "loaded".
pub fn build_all() -> Result<Vec<RegistryRow>, DecodeError> {
    let mut rows = Vec::new();

    rows.push(RegistryRow {
        rule_id: "SEC-1.1",
        validator: Box::new(InlineSecretsValidator::new()?),
    });
    rows.push(RegistryRow {
        rule_id: "SEC-1.2",
        validator: Box::new(SensitiveFilesValidator::new()?),
    });

    for spec in generic_scanner::specs()? {
        let rule_id = spec.rule_id;
        rows.push(RegistryRow {
            rule_id,
            validator: Box::new(SpecValidator::new(spec)?),
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
        assert_eq!(rows.len(), 22);
        Ok(())
    }
}
