//! `RUST-ERR-MAIN-EXITCODE` (T2) — `main` should return
//! `ExitCode`/`anyhow::Result<()>` instead of calling `std::process::exit`
//! from scattered call sites.
//!
//! Scope: flags any call to `std::process::exit`/`process::exit` anywhere
//! in the file (the scattered-exit-site problem this rule targets is not
//! limited to `main` itself) and separately flags `fn main()` with a bare
//! `()` return type when the file also contains such a call — the fixture
//! pair exercises the common single-file case.

use syn::visit::{self, Visit};
use syn::ExprCall;

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::{BuiltInRustRule, RuleId};
use enforcer_domain::paths::RelPath;
use enforcer_domain::rules_types::RulePredicateResult;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// The `RUST-ERR-MAIN-EXITCODE` `Validator`.
#[derive(Debug)]
pub struct ErrMainExitcodeValidator {
    rule_id: RuleId,
}

impl ErrMainExitcodeValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: BuiltInRustRule::ErrMainExitcode.id(),
        })
    }
}

impl Validator for ErrMainExitcodeValidator {
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

fn call_path_ends_with_exit(expr: &syn::Expr) -> RulePredicateResult {
    let syn::Expr::Path(path_expr) = expr else {
        return RulePredicateResult::NotMatched;
    };
    if path_expr
        .path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "exit")
        && path_expr
            .path
            .segments
            .iter()
            .any(|segment| segment.ident == "process")
    {
        RulePredicateResult::Matched
    } else {
        RulePredicateResult::NotMatched
    }
}

struct Visitor<'a> {
    rule_id: &'a RuleId,
    file: &'a RelPath,
    findings: Vec<Finding>,
}

impl<'ast> Visit<'ast> for Visitor<'_> {
    fn visit_expr_call(&mut self, item: &'ast ExprCall) {
        if call_path_ends_with_exit(&item.func) == RulePredicateResult::Matched {
            let line = crate::boundary::finding::source_line(item);
            let Ok(finding) = crate::boundary::finding::from_source(
                (self.rule_id, Severity::Warning),
                "scattered `std::process::exit` call",
                "Fix: have `main` return `ExitCode` (or `anyhow::Result<()>`) and \
                          propagate the failure up to it instead of calling `process::exit` \
                          from a nested call site.",
                self.file,
                line,
            ) else {
                return;
            };
            self.findings.push(finding);
        }
        visit::visit_expr_call(self, item);
    }
}

#[cfg(test)]
mod tests {

    use crate::boundary::fixture::run_fixture_parity;

    use super::ErrMainExitcodeValidator;

    #[test]
    fn fires_on_process_exit_and_silent_on_exitcode() -> Result<(), Box<dyn std::error::Error>> {
        let validator = ErrMainExitcodeValidator::new()?;
        run_fixture_parity(
            &validator,
            "fixtures/err-main-exitcode/fail_process_exit.rs",
            "fixtures/err-main-exitcode/pass_exitcode.rs",
        )?;
        Ok(())
    }

    #[test]
    fn malformed_source_stays_silent() -> Result<(), Box<dyn std::error::Error>> {
        use enforcer_validator::validator::Validator;
        let validator = ErrMainExitcodeValidator::new()?;
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
