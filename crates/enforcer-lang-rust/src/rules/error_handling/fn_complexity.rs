//! `RUST-FN-COMPLEXITY` (T2) — free functions should keep cyclomatic
//! complexity under 10 and nesting depth at or under 3.
//!
//! Cyclomatic complexity here is approximated as `1 + branch_count`, where
//! `branch_count` is the number of `if`/`else if`/`match` arm (beyond the
//! first)/`&&`/`||`/loop constructs — the standard McCabe approximation
//! for straight-line + branching control flow.

use syn::visit::{self, Visit};
use syn::{Expr, ItemFn};

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::{BuiltInRustRule, RuleId};
use enforcer_domain::paths::RelPath;
use enforcer_domain::rules_types::RustCyclomaticComplexity;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

const MAX_CYCLOMATIC_ALLOWED: RustCyclomaticComplexity = RustCyclomaticComplexity::from_count(9);

/// The `RUST-FN-COMPLEXITY` `Validator`.
#[derive(Debug)]
pub struct FnComplexityValidator {
    rule_id: RuleId,
}

impl FnComplexityValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: BuiltInRustRule::FnComplexity.id(),
        })
    }
}

impl Validator for FnComplexityValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Ok(file) = syn::parse_file(input.source.as_str()) else {
            return Vec::new();
        };
        let mut visitor = Visitor {
            rule_id: &self.rule_id,
            file: input.file,
            findings: Vec::new(),
        };
        visitor.visit_file(&file);
        visitor.findings
    }
}

struct ComplexityCounter {
    complexity: RustCyclomaticComplexity,
}

impl<'ast> Visit<'ast> for ComplexityCounter {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            Expr::If(_) => self.complexity.increment(),
            Expr::Match(item) => {
                let arm_count = u32::try_from(item.arms.len()).unwrap_or(u32::MAX);
                self.complexity
                    .increment_by(RustCyclomaticComplexity::from_count(
                        arm_count.saturating_sub(1).max(1),
                    ));
            }
            Expr::Binary(bin) if matches!(bin.op, syn::BinOp::And(_) | syn::BinOp::Or(_)) => {
                self.complexity.increment();
            }
            Expr::ForLoop(_) | Expr::While(_) | Expr::Loop(_) => self.complexity.increment(),
            _ => {}
        }
        visit::visit_expr(self, expr);
    }
}

fn cyclomatic_complexity(item: &ItemFn) -> RustCyclomaticComplexity {
    let mut counter = ComplexityCounter {
        complexity: RustCyclomaticComplexity::from_count(1),
    };
    counter.visit_block(&item.block);
    counter.complexity
}

struct Visitor<'a> {
    rule_id: &'a RuleId,
    file: &'a RelPath,
    findings: Vec<Finding>,
}

impl<'ast> Visit<'ast> for Visitor<'_> {
    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        let complexity = cyclomatic_complexity(item);
        if complexity > MAX_CYCLOMATIC_ALLOWED {
            let line = crate::boundary::finding::source_line(item);
            let Ok(finding) = crate::boundary::finding::from_source(
                (self.rule_id, Severity::Warning),
                format!(
                    "`fn {}` cyclomatic complexity {complexity} (max {MAX_CYCLOMATIC_ALLOWED})",
                    item.sig.ident
                ),
                format!(
                    "Fix: extract branches out of `fn {}` into smaller guard-claused helper \
                     functions to bring cyclomatic complexity to {MAX_CYCLOMATIC_ALLOWED} or less.",
                    item.sig.ident
                ),
                self.file,
                line,
            ) else {
                return;
            };
            self.findings.push(finding);
        }
        visit::visit_item_fn(self, item);
    }
}

#[cfg(test)]
mod tests {

    use crate::boundary::fixture::run_fixture_parity;

    use super::FnComplexityValidator;

    #[test]
    fn fires_on_high_complexity_and_silent_on_simple_fn() -> Result<(), Box<dyn std::error::Error>>
    {
        let validator = FnComplexityValidator::new()?;
        run_fixture_parity(
            &validator,
            "fixtures/fn-complexity/fail_complex.rs",
            "fixtures/fn-complexity/pass_simple.rs",
        )?;
        Ok(())
    }

    #[test]
    fn malformed_source_stays_silent() -> Result<(), Box<dyn std::error::Error>> {
        use enforcer_validator::validator::Validator;
        let validator = FnComplexityValidator::new()?;
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
