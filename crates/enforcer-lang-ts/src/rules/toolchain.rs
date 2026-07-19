//! `typescript/toolchain` — 4 rules that inspect `tsconfig*.json`,
//! `package.json`, and CI workflow text for required compiler/CI wiring:
//!
//! - TS-5.1: `tsc --noEmit` must run through the harness (`compilerOptions`
//!   present in a `tsconfig.json`-shaped file, and the CI/script wiring
//!   text references `tsc --noEmit`).
//! - TS-7.1: `tsconfig.json`'s `compilerOptions.strict` must not be `false`.
//! - TS-7.12: CI/script wiring must use `npm ci`, not `npm install`, for
//!   the install step.
//! - TS-7.13: ESLint config must enable the unsafe-TypeScript rule trio
//!   (`no-floating-promises`, `no-explicit-any`, `no-unsafe-*`).
//!
//! These are text/JSON-shape checks over config and CI files, not live
//! `tsc`/`eslint` execution — running the real compiler/linter is an
//! `enforcer-harness` concern (see arc-18).

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::boundary::finding::{from_source, SourceFinding, FIRST_SOURCE_LINE};
use crate::boundary::toolchain_policy::ToolchainRule;

/// One validator per canonical toolchain-rule variant.
#[derive(Debug)]
#[doc = "TypeScript toolchain policy validator."]
pub struct ToolchainValidator {
    rule_id: RuleId,
    rule: ToolchainRule,
}

impl ToolchainValidator {
    fn new_for(
        rule: ToolchainRule,
    ) -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        let rule_id = crate::boundary::rule_spec::decode_rule_id(rule.rule_id())?;
        Ok(Self { rule_id, rule })
    }

    /// TS-5.1.
    pub fn ts_5_1() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Self::new_for(ToolchainRule::Ts5_1)
    }

    /// TS-7.1.
    pub fn ts_7_1() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Self::new_for(ToolchainRule::Ts7_1)
    }

    /// TS-7.12.
    pub fn ts_7_12() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Self::new_for(ToolchainRule::Ts7_12)
    }

    /// TS-7.13.
    pub fn ts_7_13() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Self::new_for(ToolchainRule::Ts7_13)
    }
}

impl Validator for ToolchainValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        if !self.rule.fires(input.source.as_str()) {
            return Vec::new();
        }
        from_source(
            &self.rule_id,
            input.file,
            SourceFinding {
                severity: Severity::Error,
                title: self.rule.title(),
                detail: format!("toolchain policy `{}` violated", self.rule.rule_id()),
                line: FIRST_SOURCE_LINE,
                snippet: None,
            },
        )
        .into_iter()
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::ToolchainValidator;
    use crate::boundary::test_fixtures::run_fixture_parity;

    #[test]
    fn ts_5_1_requires_tsc_no_emit_wiring() -> Result<(), Box<dyn std::error::Error>> {
        let validator = ToolchainValidator::ts_5_1()?;
        run_fixture_parity(
            &validator,
            "fixtures/toolchain/ts-5-1/fail.json",
            "fixtures/toolchain/ts-5-1/pass.json",
        )?;
        Ok(())
    }

    #[test]
    fn ts_7_1_forbids_strict_false() -> Result<(), Box<dyn std::error::Error>> {
        let validator = ToolchainValidator::ts_7_1()?;
        run_fixture_parity(
            &validator,
            "fixtures/toolchain/ts-7-1/fail.json",
            "fixtures/toolchain/ts-7-1/pass.json",
        )?;
        Ok(())
    }

    #[test]
    fn ts_7_12_requires_npm_ci() -> Result<(), Box<dyn std::error::Error>> {
        let validator = ToolchainValidator::ts_7_12()?;
        run_fixture_parity(
            &validator,
            "fixtures/toolchain/ts-7-12/fail.yml",
            "fixtures/toolchain/ts-7-12/pass.yml",
        )?;
        Ok(())
    }

    #[test]
    fn ts_7_13_requires_unsafe_rule_trio() -> Result<(), Box<dyn std::error::Error>> {
        let validator = ToolchainValidator::ts_7_13()?;
        run_fixture_parity(
            &validator,
            "fixtures/toolchain/ts-7-13/fail.json",
            "fixtures/toolchain/ts-7-13/pass.json",
        )?;
        Ok(())
    }
}
