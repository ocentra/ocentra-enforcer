//! `RUST-SAFETY-COMMENT` — every `unsafe` block needs a `// SAFETY:`
//! comment immediately above it.
//!
//! `unsafe_code = "forbid"` is the workspace's own lint, but this rule
//! governs CONSUMER repos that have not opted into that forbid-wall: any
//! `unsafe { ... }` block must be preceded by a `// SAFETY:` line comment
//! explaining the invariant the caller is upholding.

use syn::visit::{self, Visit};
use syn::ExprUnsafe;

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::{BuiltInRustRule, RuleId};
use enforcer_domain::paths::RelPath;
use enforcer_domain::rules_types::RulePredicateResult;
use enforcer_domain::severity::Severity;
use enforcer_domain::telemetry_types::SourceLine;
use enforcer_validator::validator::{ValidationInput, Validator};

/// The `RUST-SAFETY-COMMENT` `Validator`.
#[derive(Debug)]
pub struct SafetyCommentValidator {
    rule_id: RuleId,
}

impl SafetyCommentValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: BuiltInRustRule::SafetyComment.id(),
        })
    }
}

impl Validator for SafetyCommentValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Ok(file) = syn::parse_file(input.source.as_str()) else {
            return Vec::new();
        };
        // Line-based SAFETY-comment lookup: for each `unsafe { ... }`
        // block's start line, check whether the immediately preceding
        // non-blank source line contains `SAFETY:`. `syn` discards regular
        // comments from its AST, so this rule intentionally cross-checks
        // against the raw source text rather than the parsed tree.
        let mut visitor = Visitor {
            rule_id: &self.rule_id,
            file: input.file,
            findings: Vec::new(),
            source: input.source,
        };
        visitor.visit_file(&file);
        visitor.findings
    }
}

struct Visitor<'a, 's> {
    rule_id: &'a RuleId,
    file: &'a RelPath,
    findings: Vec<Finding>,
    source: enforcer_domain::boundary::validation::ValidationSource<'s>,
}

impl<'s> Visitor<'_, 's> {
    fn has_safety_comment_above(&self, line: SourceLine) -> RulePredicateResult {
        // Walk upward through the contiguous `//`-comment block
        // immediately preceding the `unsafe` block (skipping blank lines
        // between the block start and the code line), checking each
        // comment line for `SAFETY:` — a multi-line `// SAFETY: ...` /
        // `// continuation...` block only needs the marker on ONE of its
        // lines, not necessarily the line directly touching `unsafe`.
        let mut idx = line.value().get().saturating_sub(2); // convert to 0-based, step back one
        let mut seen_comment = false;
        loop {
            let Ok(index) = usize::try_from(idx) else {
                return RulePredicateResult::NotMatched;
            };
            let Some(text) = self.source.as_str().lines().nth(index) else {
                return RulePredicateResult::NotMatched;
            };
            let trimmed = text.trim();
            if trimmed.is_empty() {
                if seen_comment || idx == 0 {
                    return RulePredicateResult::NotMatched;
                }
                if idx == 0 {
                    return RulePredicateResult::NotMatched;
                }
                idx -= 1;
                continue;
            }
            if !trimmed.starts_with("//") {
                return RulePredicateResult::NotMatched;
            }
            if trimmed.contains("SAFETY:") {
                return RulePredicateResult::Matched;
            }
            seen_comment = true;
            if idx == 0 {
                return RulePredicateResult::NotMatched;
            }
            idx -= 1;
        }
    }
}

impl<'ast, 's> Visit<'ast> for Visitor<'_, 's> {
    fn visit_expr_unsafe(&mut self, item: &'ast ExprUnsafe) {
        let line = crate::boundary::finding::source_line(item);
        if self.has_safety_comment_above(line) == RulePredicateResult::NotMatched {
            let Ok(finding) = crate::boundary::finding::from_source(
                (self.rule_id, Severity::Error),
                "`unsafe` block with no `// SAFETY:` comment",
                "Fix: add a `// SAFETY: ...` comment immediately above this `unsafe` \
                          block explaining the invariant the caller upholds.",
                self.file,
                line,
            ) else {
                return;
            };
            self.findings.push(finding);
        }
        visit::visit_expr_unsafe(self, item);
    }
}

#[cfg(test)]
mod tests {

    use crate::boundary::fixture::run_fixture_parity;

    use super::SafetyCommentValidator;

    #[test]
    fn fires_on_missing_safety_comment_and_silent_when_present(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = SafetyCommentValidator::new()?;
        run_fixture_parity(
            &validator,
            "fixtures/safety-comment/fail_unsafe.rs",
            "fixtures/safety-comment/pass_unsafe.rs",
        )?;
        Ok(())
    }

    #[test]
    fn malformed_source_stays_silent() -> Result<(), Box<dyn std::error::Error>> {
        use enforcer_validator::validator::Validator;
        let validator = SafetyCommentValidator::new()?;
        let file: enforcer_domain::paths::RelPath =
            crate::boundary::fixture::source_file("crates/x/src/lib.rs")?;
        let findings = validator.validate(enforcer_validator::validator::ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(
                "malformed rust {{{",
            ),
            scope: enforcer_domain::findings::ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }
}
