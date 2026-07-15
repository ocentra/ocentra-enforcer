//! The full TS-family registry: every one of the 73 `TS-*` rule ids paired
//! with its constructed [`Validator`]. This is the single source
//! `tests/completeness.rs` walks to prove count-parity against
//! `rules/rules.json`'s `language == "typescript"` count (73) — one entry
//! per rule id, no orphans, no duplicates.

use enforcer_domain::boundary::decode_error::DecodeError;
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
    /// The rule id this row proves, e.g. `TS-6.1`.
    pub rule_id: &'static str,
    /// The constructed validator for this rule.
    pub validator: Box<dyn Validator>,
}

/// Build every one of the 73 TS-family rows. Fails closed (propagates the
/// first construction error) rather than silently dropping a malformed
/// entry — a registry that failed to build completely must not be treated
/// as "loaded".
pub fn build_all() -> Result<Vec<RegistryRow>, DecodeError> {
    let mut rows = Vec::new();

    for spec in source_scan::SPECS {
        rows.push(RegistryRow {
            rule_id: spec.rule_id,
            validator: Box::new(SpecValidator::new(*spec)?),
        });
    }

    rows.push(RegistryRow {
        rule_id: "TS-3.1",
        validator: Box::new(TestScanValidator::new()?),
    });

    rows.push(RegistryRow {
        rule_id: "TS-4.1",
        validator: Box::new(ImportBoundariesValidator::new()?),
    });

    rows.push(RegistryRow {
        rule_id: "TS-5.1",
        validator: Box::new(ToolchainValidator::ts_5_1()?),
    });
    rows.push(RegistryRow {
        rule_id: "TS-7.1",
        validator: Box::new(ToolchainValidator::ts_7_1()?),
    });
    rows.push(RegistryRow {
        rule_id: "TS-7.12",
        validator: Box::new(ToolchainValidator::ts_7_12()?),
    });
    rows.push(RegistryRow {
        rule_id: "TS-7.13",
        validator: Box::new(ToolchainValidator::ts_7_13()?),
    });

    rows.push(RegistryRow {
        rule_id: "TS-5.2",
        validator: Box::new(EslintJsonValidator::new()?),
    });

    rows.push(RegistryRow {
        rule_id: "TS-8.10",
        validator: Box::new(DecoderNegativeCaseValidator::new()?),
    });

    for spec in generic_scanner::SPECS {
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
