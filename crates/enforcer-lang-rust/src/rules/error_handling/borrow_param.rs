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

use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{FnArg, ItemFn, Pat, Type};

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// The `RUST-BORROW-1.1` `Validator`.
pub struct BorrowParamValidator {
    rule_id: RuleId,
}

impl BorrowParamValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: "RUST-BORROW-1.1".parse()?,
        })
    }
}

impl Validator for BorrowParamValidator {
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

fn param_name_and_owned_string(arg: &FnArg) -> Option<String> {
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
    Some(pat_ident.ident.to_string())
}

struct Visitor {
    rule_id: RuleId,
    file: RelPath,
    findings: Vec<Finding>,
}

impl<'ast> Visit<'ast> for Visitor {
    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        for arg in &item.sig.inputs {
            if let Some(name) = param_name_and_owned_string(arg) {
                let line = u32::try_from(arg.span().start().line.max(1)).unwrap_or(u32::MAX);
                self.findings.push(Finding {
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Warning,
                    title: format!("param `{name}: String` taken by value"),
                    detail: format!(
                        "Fix: if `fn {}` only reads `{name}`, borrow it as `&str` instead of \
                         taking ownership of a `String`.",
                        item.sig.ident
                    ),
                    file: self.file.clone(),
                    line,
                    snippet: None,
                });
            }
        }
        visit::visit_item_fn(self, item);
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;

    use super::BorrowParamValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn fires_on_owned_string_and_silent_on_borrowed_str() -> Result<(), Box<dyn std::error::Error>>
    {
        let validator = BorrowParamValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "fixtures/borrow-param/fail_owned_string.rs",
            "fixtures/borrow-param/pass_borrowed_str.rs",
        )?;
        Ok(())
    }

    #[test]
    fn unparseable_source_stays_silent() -> Result<(), Box<dyn std::error::Error>> {
        use enforcer_validator::validator::Validator;
        let validator = BorrowParamValidator::new()?;
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
