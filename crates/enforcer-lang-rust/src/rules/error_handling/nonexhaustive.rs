//! `RUST-ERR-NONEXHAUSTIVE` — public error enums must carry
//! `#[non_exhaustive]`.
//!
//! A public error enum without `#[non_exhaustive]` locks downstream
//! consumers into exhaustive matches, making adding a new error variant a
//! breaking change. This rule flags any `pub enum` whose name ends in
//! `Error` and that lacks the attribute.

use syn::visit::{self, Visit};
use syn::{Attribute, ItemEnum, Visibility};

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::{BuiltInRustRule, RuleId};
use enforcer_domain::paths::RelPath;
use enforcer_domain::rules_types::RulePredicateResult;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// The `RUST-ERR-NONEXHAUSTIVE` `Validator`.
#[derive(Debug)]
pub struct NonExhaustiveValidator {
    rule_id: RuleId,
}

impl NonExhaustiveValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: BuiltInRustRule::ErrNonExhaustive.id(),
        })
    }
}

impl Validator for NonExhaustiveValidator {
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

fn is_public(vis: &Visibility) -> RulePredicateResult {
    if matches!(vis, Visibility::Public(_)) {
        RulePredicateResult::Matched
    } else {
        RulePredicateResult::NotMatched
    }
}

fn has_non_exhaustive(attrs: &[Attribute]) -> RulePredicateResult {
    if attrs
        .iter()
        .any(|attr| attr.path().is_ident("non_exhaustive"))
    {
        RulePredicateResult::Matched
    } else {
        RulePredicateResult::NotMatched
    }
}

/// Error-shaped enum name heuristic: ends in `Error`. Scoping to this
/// naming convention avoids false positives on ordinary closed enums that
/// are not part of a public error surface.
fn looks_like_error_enum(name: &syn::Ident) -> RulePredicateResult {
    if crate::boundary::syntax::ident_ends_with(name, "Error") {
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
    fn visit_item_enum(&mut self, item: &'ast ItemEnum) {
        if is_public(&item.vis) == RulePredicateResult::Matched
            && looks_like_error_enum(&item.ident) == RulePredicateResult::Matched
            && has_non_exhaustive(&item.attrs) == RulePredicateResult::NotMatched
        {
            let line = crate::boundary::finding::source_line(item);
            let Ok(finding) = crate::boundary::finding::from_source(
                (self.rule_id, Severity::Error),
                "public error enum missing #[non_exhaustive]",
                format!(
                    "Fix: add `#[non_exhaustive]` above `pub enum {}` so adding a new \
                     variant later is not a breaking change for downstream consumers.",
                    item.ident
                ),
                self.file,
                line,
            ) else {
                return;
            };
            self.findings.push(finding);
        }
        visit::visit_item_enum(self, item);
    }
}

#[cfg(test)]
mod tests {

    use crate::boundary::fixture::run_fixture_parity;
    use enforcer_validator::validator::Validator;

    use super::NonExhaustiveValidator;

    #[test]
    fn fires_on_missing_non_exhaustive_and_silent_when_present(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = NonExhaustiveValidator::new()?;
        run_fixture_parity(
            &validator,
            "fixtures/nonexhaustive/fail_enum.rs",
            "fixtures/nonexhaustive/pass_enum.rs",
        )?;
        Ok(())
    }

    #[test]
    fn malformed_source_stays_silent() -> Result<(), Box<dyn std::error::Error>> {
        let validator = NonExhaustiveValidator::new()?;
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
