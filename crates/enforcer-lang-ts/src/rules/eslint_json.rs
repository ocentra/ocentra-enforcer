//! `typescript/eslint-json` — TS-5.2 (ESLint JSON diagnostics must pass).
//! Checks that the project wires ESLint through the `typescript-eslint`
//! parser/plugin and runs it with `--format json` (the shape the Enforcer
//! harness consumes), rather than running unconfigured/default-format
//! ESLint.

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::boundary::finding::{from_source, SourceFinding, FIRST_SOURCE_LINE};

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
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: crate::boundary::rule_spec::decode_rule_id(RULE_ID)?,
        })
    }
}

impl Validator for EslintJsonValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let has_typescript_eslint = input.source.as_str().contains("typescript-eslint");
        let has_json_format = input.source.as_str().contains("--format json")
            || input.source.as_str().contains("\"json\"");
        if has_typescript_eslint && has_json_format {
            return Vec::new();
        }
        from_source(
            &self.rule_id,
            input.file,
            SourceFinding {
                severity: Severity::Error,
                title: "ESLint JSON diagnostics must pass",
                // ALLOC-JUSTIFICATION: the canonical Finding owns diagnostic
                // detail after this borrowed validator invocation returns.
                detail: "ESLint wiring must use typescript-eslint and emit --format json"
                    .to_owned(),
                line: FIRST_SOURCE_LINE,
                snippet: None,
            },
        )
        .into_iter()
        .collect()
    }
}
