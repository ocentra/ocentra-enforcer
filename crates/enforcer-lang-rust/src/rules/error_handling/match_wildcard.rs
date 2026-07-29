//! `RUST-MATCH-NO-WILDCARD` — no catch-all `_ =>` arm on a `match` over an
//! internal (locally-defined) enum; prefer exhaustive per-variant arms so
//! the compiler forces every call site to handle a newly added variant.
//!
//! Scope: this validator only flags a wildcard arm when the file also
//! defines at least one local `enum` — it has no cross-file type
//! resolution, so it cannot tell whether a given `match` scrutinee is
//! actually of a local enum type. Restricting to files that define an enum
//! keeps the false-positive rate low (matches over `Result`/`Option`/
//! external types in a file with no local enum stay unflagged) while still
//! catching the common single-file case the fixture pair exercises.

use syn::visit::{self, Visit};
use syn::{Arm, ExprMatch, ItemEnum, Pat};

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::{BuiltInRustRule, RuleId};
use enforcer_domain::paths::RelPath;
use enforcer_domain::rules_types::RulePredicateResult;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// The `RUST-MATCH-NO-WILDCARD` `Validator`.
#[derive(Debug)]
pub struct MatchWildcardValidator {
    rule_id: RuleId,
}

impl MatchWildcardValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: BuiltInRustRule::MatchNoWildcard.id(),
        })
    }
}

impl Validator for MatchWildcardValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Ok(file) = syn::parse_file(input.source.as_str()) else {
            return Vec::new();
        };

        let mut has_local_enum = HasLocalEnum(RulePredicateResult::NotMatched);
        has_local_enum.visit_file(&file);
        if has_local_enum.0 == RulePredicateResult::NotMatched {
            return Vec::new();
        }

        let mut visitor = Visitor {
            rule_id: &self.rule_id,
            file: input.file,
            findings: Vec::new(),
        };
        visitor.visit_file(&file);
        visitor.findings
    }
}

struct HasLocalEnum(RulePredicateResult);

impl<'ast> Visit<'ast> for HasLocalEnum {
    fn visit_item_enum(&mut self, _item: &'ast ItemEnum) {
        self.0 = RulePredicateResult::Matched;
    }
}

fn arm_is_wildcard(arm: &Arm) -> RulePredicateResult {
    if matches!(arm.pat, Pat::Wild(_)) {
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
    fn visit_expr_match(&mut self, item: &'ast ExprMatch) {
        if item
            .arms
            .iter()
            .any(|arm| arm_is_wildcard(arm) == RulePredicateResult::Matched)
        {
            let line = crate::boundary::finding::source_line(item);
            let Ok(finding) = crate::boundary::finding::from_source(
                (self.rule_id, Severity::Error),
                "catch-all `_ =>` arm on a match over a local enum",
                "Fix: replace the `_ =>` catch-all with exhaustive per-variant arms so \
                          the compiler forces this match to be updated when a new variant is \
                          added.",
                self.file,
                line,
            ) else {
                return;
            };
            self.findings.push(finding);
        }
        visit::visit_expr_match(self, item);
    }
}

#[cfg(test)]
mod tests {

    use crate::boundary::fixture::run_fixture_parity;

    use super::MatchWildcardValidator;

    #[test]
    fn fires_on_wildcard_arm_and_silent_on_exhaustive_arms(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = MatchWildcardValidator::new()?;
        run_fixture_parity(
            &validator,
            "fixtures/match-wildcard/fail_wildcard.rs",
            "fixtures/match-wildcard/pass_exhaustive.rs",
        )?;
        Ok(())
    }

    #[test]
    fn malformed_source_stays_silent() -> Result<(), Box<dyn std::error::Error>> {
        use enforcer_validator::validator::Validator;
        let validator = MatchWildcardValidator::new()?;
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
