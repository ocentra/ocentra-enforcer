//! `typescript/test-scan` — TS-3.1 (skipped/focused JS/TS tests are
//! forbidden). Scans test files (`*.test.ts`, `*.spec.ts`, `__tests__/**`)
//! for `.skip`/`.only` suite/test modifiers and the weak
//! `expect(true).toBe(true)` assertion shape.
//!
//! Position guard (mem-arc-06-0002): `.skip`/`.only` must follow
//! `describe`/`test`/`it` directly (dotted-call position), not merely
//! appear anywhere in the file (e.g. a variable named `skipCount`).

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::boundary::finding::{from_source, SourceFinding};
use crate::boundary::source_analysis::{has_skip_or_only_suite_modifier, has_weak_assertion};

use crate::boundary::source_text::{lines, source_line_role, SourceLineRole};

const RULE_ID: &str = "TS-3.1";

/// `typescript/test-scan` validator for TS-3.1.
#[derive(Debug)]
#[doc = "TypeScript test-quality validator."]
pub struct TestScanValidator {
    rule_id: RuleId,
}

impl TestScanValidator {
    /// Build the validator. Fails closed if the literal `TS-3.1` somehow
    /// stops parsing as a [`RuleId`] (never true in a passing build; see
    /// `tests/completeness.rs`).
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: crate::boundary::rule_spec::decode_rule_id(RULE_ID)?,
        })
    }
}

impl Validator for TestScanValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        for line in lines(input.source) {
            if source_line_role(line.text) == SourceLineRole::CommentOnly {
                continue;
            }
            if has_skip_or_only_suite_modifier(line.text.as_str())
                || has_weak_assertion(line.text.as_str())
            {
                findings.extend(from_source(
                    &self.rule_id,
                    input.file,
                    SourceFinding {
                        severity: Severity::Error,
                        title: "Skipped/focused JavaScript tests are forbidden",
                        detail: format!(
                            "line {} skips/focuses a suite or asserts weakly",
                            line.number
                        ),
                        line: line.number,
                        snippet: Some(line.text.as_str().trim()),
                    },
                ));
            }
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use super::TestScanValidator;
    use crate::boundary::test_fixtures::run_fixture_parity;

    #[test]
    fn fires_on_skip_and_stays_silent_on_clean_test() -> Result<(), Box<dyn std::error::Error>> {
        let validator = TestScanValidator::new()?;
        run_fixture_parity(
            &validator,
            "fixtures/test-scan/ts-3-1/fail.test.ts",
            "fixtures/test-scan/ts-3-1/pass.test.ts",
        )?;
        Ok(())
    }
}
