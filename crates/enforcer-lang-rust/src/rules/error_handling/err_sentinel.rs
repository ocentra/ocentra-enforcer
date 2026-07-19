//! `RUST-ERR-SENTINEL` (T2) — a function should not signal "not found"/
//! "no result" with a magic sentinel value (`-1`, `0`, empty string) in an
//! otherwise-normal integer/string return type; use `Option<T>`/
//! `Result<T, E>` instead.
//!
//! Heuristic: flags a free function returning a plain signed integer type
//! (`i8`/`i16`/`i32`/`i64`/`isize`) whose body's LAST statement (in any
//! control-flow position, most simply its trailing tail expression) is a
//! literal negative-one. This intentionally narrow heuristic keeps false
//! positives low; it is not a general dataflow sentinel detector.

use syn::visit::{self, Visit};
use syn::{Expr, ItemFn, ReturnType, Stmt, Type};

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::{BuiltInRustRule, RuleId};
use enforcer_domain::paths::RelPath;
use enforcer_domain::rules_types::RulePredicateResult;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

const SIGNED_INT_TYPES: &[&str] = &["i8", "i16", "i32", "i64", "isize"];

/// The `RUST-ERR-SENTINEL` `Validator`.
#[derive(Debug)]
pub struct ErrSentinelValidator {
    rule_id: RuleId,
}

impl ErrSentinelValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: BuiltInRustRule::ErrSentinel.id(),
        })
    }
}

impl Validator for ErrSentinelValidator {
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

fn returns_plain_signed_int(output: &ReturnType) -> RulePredicateResult {
    let ReturnType::Type(_, ty) = output else {
        return RulePredicateResult::NotMatched;
    };
    let Type::Path(type_path) = ty.as_ref() else {
        return RulePredicateResult::NotMatched;
    };
    if type_path
        .path
        .segments
        .last()
        .is_some_and(|segment| SIGNED_INT_TYPES.iter().any(|name| segment.ident == *name))
    {
        RulePredicateResult::Matched
    } else {
        RulePredicateResult::NotMatched
    }
}

fn is_negative_one_literal(expr: &Expr) -> RulePredicateResult {
    if let Expr::Unary(unary) = expr {
        if matches!(unary.op, syn::UnOp::Neg(_)) {
            if let Expr::Lit(lit) = unary.expr.as_ref() {
                if let syn::Lit::Int(int_lit) = &lit.lit {
                    return if int_lit.base10_digits() == "1" {
                        RulePredicateResult::Matched
                    } else {
                        RulePredicateResult::NotMatched
                    };
                }
            }
        }
    }
    RulePredicateResult::NotMatched
}

fn body_tail_is_sentinel(item: &ItemFn) -> RulePredicateResult {
    let Some(last) = item.block.stmts.last() else {
        return RulePredicateResult::NotMatched;
    };
    match last {
        Stmt::Expr(expr, None) => is_negative_one_literal(expr),
        _ => RulePredicateResult::NotMatched,
    }
}

struct Visitor<'a> {
    rule_id: &'a RuleId,
    file: &'a RelPath,
    findings: Vec<Finding>,
}

impl<'ast> Visit<'ast> for Visitor<'_> {
    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        if returns_plain_signed_int(&item.sig.output) == RulePredicateResult::Matched
            && body_tail_is_sentinel(item) == RulePredicateResult::Matched
        {
            let line = crate::boundary::finding::source_line(item);
            let Ok(finding) = crate::boundary::finding::from_source(
                (self.rule_id, Severity::Warning),
                format!(
                    "`fn {}` signals absence with a sentinel `-1`",
                    item.sig.ident
                ),
                "Fix: change the return type to `Option<T>` (or `Result<T, E>`) and \
                          return `None` instead of a magic sentinel value.",
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

    use super::ErrSentinelValidator;

    #[test]
    fn fires_on_sentinel_and_silent_on_option() -> Result<(), Box<dyn std::error::Error>> {
        let validator = ErrSentinelValidator::new()?;
        run_fixture_parity(
            &validator,
            "fixtures/err-sentinel/fail_sentinel.rs",
            "fixtures/err-sentinel/pass_option.rs",
        )?;
        Ok(())
    }

    #[test]
    fn malformed_source_stays_silent() -> Result<(), Box<dyn std::error::Error>> {
        use enforcer_validator::validator::Validator;
        let validator = ErrSentinelValidator::new()?;
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
