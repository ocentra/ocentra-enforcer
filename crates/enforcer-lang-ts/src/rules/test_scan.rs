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

use super::text_scan::{is_comment_only_line, lines};

const RULE_ID: &str = "TS-3.1";

fn has_skip_or_only_suite_modifier(text: &str) -> bool {
    for base in ["describe", "test", "it"] {
        for suffix in [".skip(", ".only(", ".todo("] {
            let needle = format!("{base}{suffix}");
            if text.contains(&needle) {
                return true;
            }
        }
    }
    false
}

fn has_weak_assertion(text: &str) -> bool {
    text.contains("expect(true).toBe(true)") || text.contains(".toBeTruthy()")
}

/// `typescript/test-scan` validator for TS-3.1.
pub struct TestScanValidator {
    rule_id: RuleId,
}

impl TestScanValidator {
    /// Build the validator. Fails closed if the literal `TS-3.1` somehow
    /// stops parsing as a [`RuleId`] (never true in a passing build; see
    /// `tests/completeness.rs`).
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: RULE_ID.parse()?,
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
            if is_comment_only_line(line.text) {
                continue;
            }
            if has_skip_or_only_suite_modifier(line.text) || has_weak_assertion(line.text) {
                findings.push(Finding {
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Error,
                    title: "Skipped/focused JavaScript tests are forbidden".to_owned(),
                    detail: format!(
                        "line {} skips/focuses a suite or asserts weakly",
                        line.number
                    ),
                    file: input.file.clone(),
                    line: line.number,
                    snippet: Some(line.text.trim().to_owned()),
                });
            }
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use super::TestScanValidator;
    use enforcer_validator::harness::run_fixture_parity;
    use std::path::PathBuf;

    #[test]
    fn fires_on_skip_and_stays_silent_on_clean_test() -> Result<(), Box<dyn std::error::Error>> {
        let validator = TestScanValidator::new()?;
        run_fixture_parity(
            &validator,
            &PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            "fixtures/test-scan/ts-3-1/fail.test.ts",
            "fixtures/test-scan/ts-3-1/pass.test.ts",
        )?;
        Ok(())
    }
}
