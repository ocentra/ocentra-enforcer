//! `RUST-FN-MAX-PARAMS` — free/assoc functions may take at most 5
//! parameters; beyond that, bundle inputs into a named struct.
//!
//! `self`/`&self`/`&mut self` receivers do not count toward the limit —
//! only the explicit typed parameter list does.

use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{FnArg, ImplItemFn, ItemFn, Signature, TraitItemFn};

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// Maximum permitted explicit (non-`self`) parameter count.
const MAX_PARAMS: usize = 5;

/// The `RUST-FN-MAX-PARAMS` `Validator`.
pub struct FnMaxParamsValidator {
    rule_id: RuleId,
}

impl FnMaxParamsValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: "RUST-FN-MAX-PARAMS".parse()?,
        })
    }
}

impl Validator for FnMaxParamsValidator {
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

fn explicit_param_count(sig: &Signature) -> usize {
    sig.inputs
        .iter()
        .filter(|arg| matches!(arg, FnArg::Typed(_)))
        .count()
}

struct Visitor {
    rule_id: RuleId,
    file: RelPath,
    findings: Vec<Finding>,
}

impl Visitor {
    fn check(&mut self, sig: &Signature, span_line: u32) {
        let count = explicit_param_count(sig);
        if count > MAX_PARAMS {
            self.findings.push(Finding {
                rule_id: self.rule_id.clone(),
                severity: Severity::Error,
                title: format!(
                    "`fn {}` has {count} parameters (max {MAX_PARAMS})",
                    sig.ident
                ),
                detail: format!(
                    "Fix: bundle these {count} parameters into one named input struct and \
                     take it by value or reference instead."
                ),
                file: self.file.clone(),
                line: span_line,
                snippet: None,
            });
        }
    }
}

impl<'ast> Visit<'ast> for Visitor {
    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        let line = u32::try_from(item.span().start().line.max(1)).unwrap_or(u32::MAX);
        self.check(&item.sig, line);
        visit::visit_item_fn(self, item);
    }

    fn visit_impl_item_fn(&mut self, item: &'ast ImplItemFn) {
        let line = u32::try_from(item.span().start().line.max(1)).unwrap_or(u32::MAX);
        self.check(&item.sig, line);
        visit::visit_impl_item_fn(self, item);
    }

    fn visit_trait_item_fn(&mut self, item: &'ast TraitItemFn) {
        let line = u32::try_from(item.span().start().line.max(1)).unwrap_or(u32::MAX);
        self.check(&item.sig, line);
        visit::visit_trait_item_fn(self, item);
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;

    use super::FnMaxParamsValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn fires_on_six_params_and_silent_on_input_struct() -> Result<(), Box<dyn std::error::Error>> {
        let validator = FnMaxParamsValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "fixtures/fn-max-params/fail_six.rs",
            "fixtures/fn-max-params/pass_input_struct.rs",
        )?;
        Ok(())
    }

    #[test]
    fn unparseable_source_stays_silent() -> Result<(), Box<dyn std::error::Error>> {
        use enforcer_validator::validator::Validator;
        let validator = FnMaxParamsValidator::new()?;
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
