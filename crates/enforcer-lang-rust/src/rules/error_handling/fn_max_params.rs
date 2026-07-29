//! `RUST-FN-MAX-PARAMS` — free/assoc functions may take at most 5
//! parameters; beyond that, bundle inputs into a named struct.
//!
//! `self`/`&self`/`&mut self` receivers do not count toward the limit —
//! only the explicit typed parameter list does.

use syn::visit::{self, Visit};
use syn::{FnArg, ImplItemFn, ItemFn, Signature, TraitItemFn};

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::{BuiltInRustRule, RuleId};
use enforcer_domain::paths::RelPath;
use enforcer_domain::rules_types::RustExplicitParameterCount;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// Maximum permitted explicit (non-`self`) parameter count.
const MAX_PARAMS: RustExplicitParameterCount = RustExplicitParameterCount::from_count(5);

/// The `RUST-FN-MAX-PARAMS` `Validator`.
#[derive(Debug)]
pub struct FnMaxParamsValidator {
    rule_id: RuleId,
}

impl FnMaxParamsValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: BuiltInRustRule::FnMaxParams.id(),
        })
    }
}

impl Validator for FnMaxParamsValidator {
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

fn explicit_param_count(sig: &Signature) -> RustExplicitParameterCount {
    RustExplicitParameterCount::from_parameters(
        sig.inputs
            .iter()
            .filter(|arg| matches!(arg, FnArg::Typed(_))),
    )
}

struct Visitor<'a> {
    rule_id: &'a RuleId,
    file: &'a RelPath,
    findings: Vec<Finding>,
}

impl Visitor<'_> {
    fn check(&mut self, sig: &Signature, span_line: enforcer_domain::telemetry_types::SourceLine) {
        let count = explicit_param_count(sig);
        if count > MAX_PARAMS {
            let Ok(finding) = crate::boundary::finding::from_source(
                (self.rule_id, Severity::Error),
                format!(
                    "`fn {}` has {count} parameters (max {MAX_PARAMS})",
                    sig.ident
                ),
                format!(
                    "Fix: bundle these {count} parameters into one named input struct and \
                     take it by value or reference instead."
                ),
                self.file,
                span_line,
            ) else {
                return;
            };
            self.findings.push(finding);
        }
    }
}

impl<'ast> Visit<'ast> for Visitor<'_> {
    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        let line = crate::boundary::finding::source_line(item);
        self.check(&item.sig, line);
        visit::visit_item_fn(self, item);
    }

    fn visit_impl_item_fn(&mut self, item: &'ast ImplItemFn) {
        let line = crate::boundary::finding::source_line(item);
        self.check(&item.sig, line);
        visit::visit_impl_item_fn(self, item);
    }

    fn visit_trait_item_fn(&mut self, item: &'ast TraitItemFn) {
        let line = crate::boundary::finding::source_line(item);
        self.check(&item.sig, line);
        visit::visit_trait_item_fn(self, item);
    }
}

#[cfg(test)]
mod tests {

    use crate::boundary::fixture::run_fixture_parity;

    use super::FnMaxParamsValidator;

    #[test]
    fn fires_on_six_params_and_silent_on_input_struct() -> Result<(), Box<dyn std::error::Error>> {
        let validator = FnMaxParamsValidator::new()?;
        run_fixture_parity(
            &validator,
            "fixtures/fn-max-params/fail_six.rs",
            "fixtures/fn-max-params/pass_input_struct.rs",
        )?;
        Ok(())
    }

    #[test]
    fn malformed_source_stays_silent() -> Result<(), Box<dyn std::error::Error>> {
        use enforcer_validator::validator::Validator;
        let validator = FnMaxParamsValidator::new()?;
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
