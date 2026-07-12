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

impl std::fmt::Debug for EslintJsonValidator {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EslintJsonValidator")
            .field("rule_id", &self.rule_id)
            .finish()
    }
}

impl EslintJsonValidator {
    /// Build the validator.
    ///
    /// `RULE_ID` is a compile-time constant, so its parse cannot receive
    /// invalid, empty, oversized, or malformed caller input. The external
    /// `tests/eslint_json.rs` fixture proves invalid ESLint wiring is rejected.
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
            // CLONE-JUSTIFICATION: each finding owns its rule identifier so it remains valid after this validator is dropped.
            rule_id: self.rule_id.clone(),
            severity: Severity::Error,
            // ALLOC-JUSTIFICATION: findings cross the validator boundary and
            // deliberately own stable diagnostic text.
            title: "ESLint JSON diagnostics must pass".to_owned(),
            detail: "ESLint wiring must use typescript-eslint and emit --format json".to_owned(),
            // CLONE-JUSTIFICATION: findings must own the input file identity because the input is borrowed.
            file: input.file.clone(),
            line: 1,
            snippet: None,
        }]
    }
}
