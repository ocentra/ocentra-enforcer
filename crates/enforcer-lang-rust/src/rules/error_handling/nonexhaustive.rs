//! `RUST-ERR-NONEXHAUSTIVE` — public error enums must carry
//! `#[non_exhaustive]`.
//!
//! A public error enum without `#[non_exhaustive]` locks downstream
//! consumers into exhaustive matches, making adding a new error variant a
//! breaking change. This rule flags any `pub enum` whose name ends in
//! `Error` and that lacks the attribute.

use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Attribute, ItemEnum, Visibility};

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// The `RUST-ERR-NONEXHAUSTIVE` `Validator`.
pub struct NonExhaustiveValidator {
    rule_id: RuleId,
}

impl NonExhaustiveValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: "RUST-ERR-NONEXHAUSTIVE".parse()?,
        })
    }
}

impl Validator for NonExhaustiveValidator {
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

fn is_public(vis: &Visibility) -> bool {
    matches!(vis, Visibility::Public(_))
}

fn has_non_exhaustive(attrs: &[Attribute]) -> bool {
    attrs
        .iter()
        .any(|attr| attr.path().is_ident("non_exhaustive"))
}

/// Error-shaped enum name heuristic: ends in `Error`. Scoping to this
/// naming convention avoids false positives on ordinary closed enums that
/// are not part of a public error surface.
fn looks_like_error_enum(name: &str) -> bool {
    name.ends_with("Error")
}

struct Visitor {
    rule_id: RuleId,
    file: RelPath,
    findings: Vec<Finding>,
}

impl<'ast> Visit<'ast> for Visitor {
    fn visit_item_enum(&mut self, item: &'ast ItemEnum) {
        if is_public(&item.vis)
            && looks_like_error_enum(&item.ident.to_string())
            && !has_non_exhaustive(&item.attrs)
        {
            let line = u32::try_from(item.span().start().line.max(1)).unwrap_or(u32::MAX);
            self.findings.push(Finding {
                rule_id: self.rule_id.clone(),
                severity: Severity::Error,
                title: "public error enum missing #[non_exhaustive]".to_owned(),
                detail: format!(
                    "Fix: add `#[non_exhaustive]` above `pub enum {}` so adding a new \
                     variant later is not a breaking change for downstream consumers.",
                    item.ident
                ),
                file: self.file.clone(),
                line,
                snippet: None,
            });
        }
        visit::visit_item_enum(self, item);
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;
    use enforcer_validator::validator::Validator;

    use super::NonExhaustiveValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn fires_on_missing_non_exhaustive_and_silent_when_present(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = NonExhaustiveValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "fixtures/nonexhaustive/fail_enum.rs",
            "fixtures/nonexhaustive/pass_enum.rs",
        )?;
        Ok(())
    }

    #[test]
    fn unparseable_source_stays_silent() -> Result<(), Box<dyn std::error::Error>> {
        let validator = NonExhaustiveValidator::new()?;
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
