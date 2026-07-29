//! `RUST-BORROW-1.1` (T2) — a function that only reads a `String`/`Vec<T>`
//! parameter (never consumes it) should borrow (`&str`/`&[T]`) instead of
//! taking ownership.
//!
//! Heuristic, intentionally narrow: flags a free-function parameter typed
//! exactly `String` (not `&String`/`&str`) — ownership of a `String`
//! parameter is the single most common avoidable-clone case, and this
//! rule does not attempt a full borrow-checker-grade usage analysis (it
//! cannot tell whether the body actually consumes the value, e.g. moves
//! it into a returned struct) — treat this as a review-triggering
//! heuristic that a genuine ownership-transfer case should waive, not an
//! infallible dataflow proof.

use syn::visit::{self, Visit};
use syn::{FnArg, ItemFn, Pat, Type};

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::{BuiltInRustRule, RuleId};
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// The `RUST-BORROW-1.1` `Validator`.
#[derive(Debug)]
pub struct BorrowParamValidator {
    rule_id: RuleId,
}

impl BorrowParamValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: BuiltInRustRule::BorrowParam.id(),
        })
    }
}

impl Validator for BorrowParamValidator {
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

fn param_name_and_owned_string(arg: &FnArg) -> Option<&syn::Ident> {
    let FnArg::Typed(pat_type) = arg else {
        return None;
    };
    let Type::Path(type_path) = pat_type.ty.as_ref() else {
        return None;
    };
    let is_string = type_path
        .path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "String");
    if !is_string {
        return None;
    }
    let Pat::Ident(pat_ident) = pat_type.pat.as_ref() else {
        return None;
    };
    Some(&pat_ident.ident)
}

struct Visitor<'a> {
    rule_id: &'a RuleId,
    file: &'a RelPath,
    findings: Vec<Finding>,
}

impl<'ast> Visit<'ast> for Visitor<'_> {
    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        for arg in &item.sig.inputs {
            if let Some(name) = param_name_and_owned_string(arg) {
                let line = crate::boundary::finding::source_line(arg);
                let Ok(finding) = crate::boundary::finding::from_source(
                    (self.rule_id, Severity::Warning),
                    format!("param `{name}: String` taken by value"),
                    format!(
                        "Fix: if `fn {}` only reads `{name}`, borrow it as `&str` instead of \
                         taking ownership of a `String`.",
                        item.sig.ident
                    ),
                    self.file,
                    line,
                ) else {
                    return;
                };
                self.findings.push(finding);
            }
        }
        visit::visit_item_fn(self, item);
    }
}

#[cfg(test)]
mod tests {

    use crate::boundary::fixture::run_fixture_parity;

    use super::BorrowParamValidator;

    #[test]
    fn fires_on_owned_string_and_silent_on_borrowed_str() -> Result<(), Box<dyn std::error::Error>>
    {
        let validator = BorrowParamValidator::new()?;
        run_fixture_parity(
            &validator,
            "fixtures/borrow-param/fail_owned_string.rs",
            "fixtures/borrow-param/pass_borrowed_str.rs",
        )?;
        Ok(())
    }

    #[test]
    fn malformed_source_stays_silent() -> Result<(), Box<dyn std::error::Error>> {
        use enforcer_validator::validator::Validator;
        let validator = BorrowParamValidator::new()?;
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
