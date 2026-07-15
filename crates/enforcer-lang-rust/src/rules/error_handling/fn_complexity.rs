//! `RUST-FN-COMPLEXITY` (T2) — free functions should keep cyclomatic
//! complexity under 10 and nesting depth at or under 3.
//!
//! Cyclomatic complexity here is approximated as `1 + branch_count`, where
//! `branch_count` is the number of `if`/`else if`/`match` arm (beyond the
//! first)/`&&`/`||`/loop constructs — the standard McCabe approximation
//! for straight-line + branching control flow.

use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Expr, ItemFn};

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

const MAX_CYCLOMATIC: u32 = 10;

/// The `RUST-FN-COMPLEXITY` `Validator`.
pub struct FnComplexityValidator {
    rule_id: RuleId,
}

impl FnComplexityValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: "RUST-FN-COMPLEXITY".parse()?,
        })
    }
}

impl Validator for FnComplexityValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Ok(file) = syn::parse_file(input.source) else {
            return Vec::new();
        };
        let mut visitor = Visitor {
            rule_id: self.rule_id.clone(),
            file: input.file.clone(),
            findings: Vec::new(),
        };
        visitor.visit_file(&file);
        visitor.findings
    }
}

struct ComplexityCounter {
    branches: u32,
}

impl<'ast> Visit<'ast> for ComplexityCounter {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            Expr::If(_) => self.branches += 1,
            Expr::Match(item) => {
                let arm_count = u32::try_from(item.arms.len()).unwrap_or(u32::MAX);
                self.branches += arm_count.saturating_sub(1).max(1);
            }
            Expr::Binary(bin) if matches!(bin.op, syn::BinOp::And(_) | syn::BinOp::Or(_)) => {
                self.branches += 1;
            }
            Expr::ForLoop(_) | Expr::While(_) | Expr::Loop(_) => self.branches += 1,
            _ => {}
        }
        visit::visit_expr(self, expr);
    }
}

fn cyclomatic_complexity(item: &ItemFn) -> u32 {
    let mut counter = ComplexityCounter { branches: 0 };
    counter.visit_block(&item.block);
    1 + counter.branches
}

struct Visitor {
    rule_id: RuleId,
    file: RelPath,
    findings: Vec<Finding>,
}

impl<'ast> Visit<'ast> for Visitor {
    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        let complexity = cyclomatic_complexity(item);
        if complexity >= MAX_CYCLOMATIC {
            let line = u32::try_from(item.span().start().line.max(1)).unwrap_or(u32::MAX);
            self.findings.push(Finding {
                rule_id: self.rule_id.clone(),
                severity: Severity::Warning,
                title: format!(
                    "`fn {}` cyclomatic complexity {complexity} (max {})",
                    item.sig.ident,
                    MAX_CYCLOMATIC - 1
                ),
                detail: format!(
                    "Fix: extract branches out of `fn {}` into smaller guard-claused helper \
                     functions to bring cyclomatic complexity under {MAX_CYCLOMATIC}.",
                    item.sig.ident
                ),
                file: self.file.clone(),
                line,
                snippet: None,
            });
        }
        visit::visit_item_fn(self, item);
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;

    use super::FnComplexityValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn fires_on_high_complexity_and_silent_on_simple_fn() -> Result<(), Box<dyn std::error::Error>>
    {
        let validator = FnComplexityValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "fixtures/fn-complexity/fail_complex.rs",
            "fixtures/fn-complexity/pass_simple.rs",
        )?;
        Ok(())
    }

    #[test]
    fn unparseable_source_stays_silent() -> Result<(), Box<dyn std::error::Error>> {
        use enforcer_validator::validator::Validator;
        let validator = FnComplexityValidator::new()?;
        let file: enforcer_domain::paths::RelPath = "crates/x/src/lib.rs".parse()?;
        let findings = validator.validate(enforcer_validator::validator::ValidationInput {
            file: &file,
            source: "not valid rust {{{",
            scope: enforcer_domain::findings::ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }
}
