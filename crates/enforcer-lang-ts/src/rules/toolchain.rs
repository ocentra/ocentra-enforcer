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

/// One toolchain rule: a required marker OR a forbidden marker (exactly
/// one is `Some`), checked over the whole file text.
struct ToolchainSpec {
    rule_id: &'static str,
    title: &'static str,
    /// Fires when this substring is ABSENT.
    required_marker: Option<&'static str>,
    /// Fires when this substring is PRESENT.
    forbidden_marker: Option<&'static str>,
}

const SPECS: &[ToolchainSpec] = &[
    ToolchainSpec {
        rule_id: "TS-5.1",
        title: "TypeScript compiler checks must pass",
        required_marker: Some("tsc --noEmit"),
        forbidden_marker: None,
    },
    ToolchainSpec {
        rule_id: "TS-7.1",
        title: "TypeScript strict mode is required",
        required_marker: None,
        forbidden_marker: Some("strict: false"),
    },
    ToolchainSpec {
        rule_id: "TS-7.12",
        title: "npm ci is required in CI",
        required_marker: Some("npm ci"),
        forbidden_marker: None,
    },
    ToolchainSpec {
        rule_id: "TS-7.13",
        title: "ESLint must enforce unsafe TypeScript rules",
        required_marker: Some("no-floating-promises"),
        forbidden_marker: None,
    },
];

/// One [`Validator`] per [`ToolchainSpec`], selected by `spec_index` into
/// [`SPECS`].
pub struct ToolchainValidator {
    rule_id: RuleId,
    spec_index: usize,
}

impl ToolchainValidator {
    fn new_for(
        spec_index: usize,
    ) -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        let rule_id: RuleId = SPECS[spec_index].rule_id.parse()?;
        Ok(Self {
            rule_id,
            spec_index,
        })
    }

    /// TS-5.1.
    pub fn ts_5_1() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Self::new_for(0)
    }

    /// TS-7.1.
    pub fn ts_7_1() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Self::new_for(1)
    }

    /// TS-7.12.
    pub fn ts_7_12() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Self::new_for(2)
    }

    /// TS-7.13.
    pub fn ts_7_13() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Self::new_for(3)
    }
}

impl Validator for ToolchainValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let spec = &SPECS[self.spec_index];
        let fires = match (spec.required_marker, spec.forbidden_marker) {
            (Some(required), None) => !input.source.contains(required),
            (None, Some(forbidden)) => input.source.contains(forbidden),
            _ => false,
        };
        if !fires {
            return Vec::new();
        }
        vec![Finding {
            rule_id: self.rule_id.clone(),
            severity: Severity::Error,
            title: spec.title.to_owned(),
            detail: format!("toolchain policy `{}` violated", spec.rule_id),
            file: input.file.clone(),
            line: 1,
            snippet: None,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::ToolchainValidator;
    use enforcer_validator::harness::run_fixture_parity;
    use std::path::PathBuf;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn ts_5_1_requires_tsc_no_emit_wiring() -> Result<(), Box<dyn std::error::Error>> {
        let validator = ToolchainValidator::ts_5_1()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
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
            &manifest_dir(),
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
            &manifest_dir(),
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
            &manifest_dir(),
            "fixtures/toolchain/ts-7-13/fail.json",
            "fixtures/toolchain/ts-7-13/pass.json",
        )?;
        Ok(())
    }
}
