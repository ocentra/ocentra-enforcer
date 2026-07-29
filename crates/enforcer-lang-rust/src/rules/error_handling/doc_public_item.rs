//! `RUST-DOC-PUBLIC-ITEM` (T2) — every public item should carry a `///`
//! doc comment.
//!
//! Scope: checks `pub fn` at the item level (free functions); the fuller
//! doctrine (`# Errors`/`# Panics` sections) is a style refinement this
//! rule does not yet enforce structurally — presence of any `///` line
//! above the item is what this rule checks.

use syn::visit::{self, Visit};
use syn::{Attribute, ItemFn, Visibility};

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::{BuiltInRustRule, RuleId};
use enforcer_domain::paths::RelPath;
use enforcer_domain::rules_types::RulePredicateResult;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// The `RUST-DOC-PUBLIC-ITEM` `Validator`.
#[derive(Debug)]
pub struct DocPublicItemValidator {
    rule_id: RuleId,
}

impl DocPublicItemValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: BuiltInRustRule::DocPublicItem.id(),
        })
    }
}

impl Validator for DocPublicItemValidator {
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

fn has_doc_comment(attrs: &[Attribute]) -> RulePredicateResult {
    if attrs.iter().any(|attr| attr.path().is_ident("doc")) {
        RulePredicateResult::Matched
    } else {
        RulePredicateResult::NotMatched
    }
}

fn is_public(vis: &Visibility) -> RulePredicateResult {
    if matches!(vis, Visibility::Public(_)) {
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
    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        if is_public(&item.vis) == RulePredicateResult::Matched
            && has_doc_comment(&item.attrs) == RulePredicateResult::NotMatched
        {
            let line = crate::boundary::finding::source_line(item);
            let Ok(finding) = crate::boundary::finding::from_source(
                (self.rule_id, Severity::Warning),
                format!("`pub fn {}` has no `///` doc comment", item.sig.ident),
                format!(
                    "Fix: add a `///` summary above `pub fn {}` (and `# Errors`/`# Panics` \
                     sections if it returns `Result` or can panic).",
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

    use super::DocPublicItemValidator;

    #[test]
    fn fires_on_missing_doc_and_silent_when_documented() -> Result<(), Box<dyn std::error::Error>> {
        let validator = DocPublicItemValidator::new()?;
        run_fixture_parity(
            &validator,
            "fixtures/doc-public-item/fail_no_doc.rs",
            "fixtures/doc-public-item/pass_documented.rs",
        )?;
        Ok(())
    }

    #[test]
    fn malformed_source_stays_silent() -> Result<(), Box<dyn std::error::Error>> {
        use enforcer_validator::validator::Validator;
        let validator = DocPublicItemValidator::new()?;
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
