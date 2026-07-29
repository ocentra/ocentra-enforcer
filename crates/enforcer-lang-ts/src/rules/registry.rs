//! The full TS-family registry: every one of the 73 `TS-*` rule ids paired
//! with its constructed [`Validator`]. This is the single source
//! `tests/completeness.rs` walks to prove count-parity against
//! `rules/rules.json`'s `language == "typescript"` count (73) — one entry
//! per rule id, no orphans, no duplicates.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::ids::RuleId;
use enforcer_validator::validator::Validator;

use super::eslint_json::EslintJsonValidator;
use super::generic_scanner;
use super::import_boundaries::ImportBoundariesValidator;
use super::source_scan;
use super::spec::SpecValidator;
use super::test_scan::TestScanValidator;
use super::tests_family::DecoderNegativeCaseValidator;
use super::toolchain::ToolchainValidator;

/// One registry row: the rule id (as the literal from its owning spec/
/// validator) paired with the constructed [`Validator`] trait object.
pub struct RegistryRow {
    /// The constructed validator for this rule.
    pub validator: Box<dyn Validator>,
}

impl RegistryRow {
    /// The canonical id owned by this row's validator.
    pub fn rule_id(&self) -> &RuleId {
        self.validator.rule_id()
    }
}

impl std::fmt::Debug for RegistryRow {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RegistryRow")
            .field("rule_id", &self.rule_id())
            .finish_non_exhaustive()
    }
}

fn registry_row(validator: Box<dyn Validator>) -> RegistryRow {
    RegistryRow { validator }
}

/// Build every one of the 73 TS-family rows. Fails closed (propagates the
/// first construction error) rather than silently dropping a malformed
/// entry — a registry that failed to build completely must not be treated
/// as "loaded".
pub fn build_all() -> Result<Vec<RegistryRow>, DecodeError> {
    let mut rows = Vec::new();

    for spec in source_scan::SPECS {
        rows.push(registry_row(Box::new(SpecValidator::new(*spec)?)));
    }

    rows.push(registry_row(Box::new(TestScanValidator::new()?)));

    rows.push(registry_row(Box::new(ImportBoundariesValidator::new()?)));

    rows.push(registry_row(Box::new(ToolchainValidator::ts_5_1()?)));
    rows.push(registry_row(Box::new(ToolchainValidator::ts_7_1()?)));
    rows.push(registry_row(Box::new(ToolchainValidator::ts_7_12()?)));
    rows.push(registry_row(Box::new(ToolchainValidator::ts_7_13()?)));

    rows.push(registry_row(Box::new(EslintJsonValidator::new()?)));

    rows.push(registry_row(Box::new(DecoderNegativeCaseValidator::new()?)));

    for spec in generic_scanner::SPECS {
        rows.push(registry_row(Box::new(SpecValidator::new(*spec)?)));
    }

    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::build_all;

    #[test]
    fn registry_builds_cleanly() -> Result<(), Box<dyn std::error::Error>> {
        let rows = build_all()?;
        assert_eq!(rows.len(), 73);
        Ok(())
    }
}
