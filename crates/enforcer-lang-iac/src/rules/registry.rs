//! The full IaC-family registry: every one of the 8 `IAC-*` rule ids
//! paired with its constructed [`Validator`]. This is the single source
//! `tests/completeness.rs` walks to prove count-parity against
//! `rules/rules.json`'s `language == "iac"` count (8) — one entry per rule
//! id, no orphans, no duplicates.

use enforcer_core::error::DecodeError;
use enforcer_validator::validator::Validator;

use super::cloudformation;
use super::kubernetes;
use super::spec::SpecValidator;
use super::terraform;

/// One registry row: the rule id (as the literal from its owning spec)
/// paired with the constructed [`Validator`] trait object.
pub struct RegistryRow {
    /// The rule id this row proves, e.g. `IAC-1.1`.
    pub rule_id: &'static str,
    /// The constructed validator for this rule.
    pub validator: Box<dyn Validator>,
}

/// Build every one of the 8 IaC-family rows. Fails closed (propagates the
/// first construction error) rather than silently dropping a malformed
/// entry — a registry that failed to build completely must not be treated
/// as "loaded".
pub fn build_all() -> Result<Vec<RegistryRow>, DecodeError> {
    let mut rows = Vec::new();

    for spec in terraform::SPECS {
        rows.push(RegistryRow {
            rule_id: spec.rule_id,
            validator: Box::new(SpecValidator::new(*spec)?),
        });
    }

    for spec in cloudformation::SPECS {
        rows.push(RegistryRow {
            rule_id: spec.rule_id,
            validator: Box::new(SpecValidator::new(*spec)?),
        });
    }

    for spec in kubernetes::SPECS {
        rows.push(RegistryRow {
            rule_id: spec.rule_id,
            validator: Box::new(SpecValidator::new(*spec)?),
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
