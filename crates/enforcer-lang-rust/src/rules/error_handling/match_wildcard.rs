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

use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Arm, ExprMatch, ItemEnum, Pat};

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// The `RUST-MATCH-NO-WILDCARD` `Validator`.
pub struct MatchWildcardValidator {
    rule_id: RuleId,
}

impl MatchWildcardValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: "RUST-MATCH-NO-WILDCARD".parse()?,
        })
    }
}

impl Validator for MatchWildcardValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Ok(file) = syn::parse_file(input.source) else {
            return Vec::new();
        };

        let mut has_local_enum = HasLocalEnum(false);
        has_local_enum.visit_file(&file);
        if !has_local_enum.0 {
            return Vec::new();
        }

        let mut visitor = Visitor {
            rule_id: self.rule_id.clone(),
            file: input.file.clone(),
            findings: Vec::new(),
        };
        visitor.visit_file(&file);
        visitor.findings
    }
}

struct HasLocalEnum(bool);

impl<'ast> Visit<'ast> for HasLocalEnum {
    fn visit_item_enum(&mut self, _item: &'ast ItemEnum) {
        self.0 = true;
    }
}

fn arm_is_wildcard(arm: &Arm) -> bool {
    matches!(arm.pat, Pat::Wild(_))
}

struct Visitor {
    rule_id: RuleId,
    file: RelPath,
    findings: Vec<Finding>,
}

impl<'ast> Visit<'ast> for Visitor {
    fn visit_expr_match(&mut self, item: &'ast ExprMatch) {
        if item.arms.iter().any(arm_is_wildcard) {
            let line = u32::try_from(item.span().start().line.max(1)).unwrap_or(u32::MAX);
            self.findings.push(Finding {
                rule_id: self.rule_id.clone(),
                severity: Severity::Error,
                title: "catch-all `_ =>` arm on a match over a local enum".to_owned(),
                detail: "Fix: replace the `_ =>` catch-all with exhaustive per-variant arms so \
                          the compiler forces this match to be updated when a new variant is \
                          added."
                    .to_owned(),
                file: self.file.clone(),
                line,
                snippet: None,
            });
        }
        visit::visit_expr_match(self, item);
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;

    use super::MatchWildcardValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn fires_on_wildcard_arm_and_silent_on_exhaustive_arms(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = MatchWildcardValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "fixtures/match-wildcard/fail_wildcard.rs",
            "fixtures/match-wildcard/pass_exhaustive.rs",
        )?;
        Ok(())
    }

    #[test]
    fn unparseable_source_stays_silent() -> Result<(), Box<dyn std::error::Error>> {
        use enforcer_validator::validator::Validator;
        let validator = MatchWildcardValidator::new()?;
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
