//! `typescript/eslint-json` — TS-5.2 (ESLint JSON diagnostics must pass).
//! Checks that the project wires ESLint through the `typescript-eslint`
//! parser/plugin and runs it with `--format json` (the shape the Enforcer
//! harness consumes), rather than running unconfigured/default-format
//! ESLint.

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

const RULE_ID: &str = "TS-5.2";

/// `typescript/eslint-json` validator for TS-5.2.
pub struct EslintJsonValidator {
    rule_id: RuleId,
}

impl EslintJsonValidator {
    /// Build the validator.
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: RULE_ID.parse()?,
        })
    }
}

impl Validator for EslintJsonValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let has_typescript_eslint = input.source.contains("typescript-eslint");
        let has_json_format =
            input.source.contains("--format json") || input.source.contains("\"json\"");
        if has_typescript_eslint && has_json_format {
            return Vec::new();
        }
        vec![Finding {
            rule_id: self.rule_id.clone(),
            severity: Severity::Error,
            title: "ESLint JSON diagnostics must pass".to_owned(),
            detail: "ESLint wiring must use typescript-eslint and emit --format json".to_owned(),
            file: input.file.clone(),
            line: 1,
            snippet: None,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::EslintJsonValidator;
    use enforcer_validator::harness::run_fixture_parity;
    use std::path::PathBuf;

    #[test]
    fn requires_typescript_eslint_and_json_format() -> Result<(), Box<dyn std::error::Error>> {
        let validator = EslintJsonValidator::new()?;
        run_fixture_parity(
            &validator,
            &PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            "fixtures/eslint-json/ts-5-2/fail.json",
            "fixtures/eslint-json/ts-5-2/pass.json",
        )?;
        Ok(())
    }
}
