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

use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Expr, ItemFn, ReturnType, Stmt, Type};

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

const SIGNED_INT_TYPES: &[&str] = &["i8", "i16", "i32", "i64", "isize"];

/// The `RUST-ERR-SENTINEL` `Validator`.
pub struct ErrSentinelValidator {
    rule_id: RuleId,
}

impl ErrSentinelValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: "RUST-ERR-SENTINEL".parse()?,
        })
    }
}

impl Validator for ErrSentinelValidator {
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

fn returns_plain_signed_int(output: &ReturnType) -> bool {
    let ReturnType::Type(_, ty) = output else {
        return false;
    };
    let Type::Path(type_path) = ty.as_ref() else {
        return false;
    };
    type_path
        .path
        .segments
        .last()
        .is_some_and(|segment| SIGNED_INT_TYPES.contains(&segment.ident.to_string().as_str()))
}

fn is_negative_one_literal(expr: &Expr) -> bool {
    if let Expr::Unary(unary) = expr {
        if matches!(unary.op, syn::UnOp::Neg(_)) {
            if let Expr::Lit(lit) = unary.expr.as_ref() {
                if let syn::Lit::Int(int_lit) = &lit.lit {
                    return int_lit.base10_digits() == "1";
                }
            }
        }
    }
    false
}

fn body_tail_is_sentinel(item: &ItemFn) -> bool {
    let Some(last) = item.block.stmts.last() else {
        return false;
    };
    match last {
        Stmt::Expr(expr, None) => is_negative_one_literal(expr),
        _ => false,
    }
}

struct Visitor {
    rule_id: RuleId,
    file: RelPath,
    findings: Vec<Finding>,
}

impl<'ast> Visit<'ast> for Visitor {
    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        if returns_plain_signed_int(&item.sig.output) && body_tail_is_sentinel(item) {
            let line = u32::try_from(item.span().start().line.max(1)).unwrap_or(u32::MAX);
            self.findings.push(Finding {
                rule_id: self.rule_id.clone(),
                severity: Severity::Warning,
                title: format!(
                    "`fn {}` signals absence with a sentinel `-1`",
                    item.sig.ident
                ),
                detail: "Fix: change the return type to `Option<T>` (or `Result<T, E>`) and \
                          return `None` instead of a magic sentinel value."
                    .to_owned(),
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

    use super::ErrSentinelValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn fires_on_sentinel_and_silent_on_option() -> Result<(), Box<dyn std::error::Error>> {
        let validator = ErrSentinelValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "fixtures/err-sentinel/fail_sentinel.rs",
            "fixtures/err-sentinel/pass_option.rs",
        )?;
        Ok(())
    }

    #[test]
    fn unparseable_source_stays_silent() -> Result<(), Box<dyn std::error::Error>> {
        use enforcer_validator::validator::Validator;
        let validator = ErrSentinelValidator::new()?;
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
