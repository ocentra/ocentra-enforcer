//! `RUST-DOC-PUBLIC-ITEM` (T2) — every public item should carry a `///`
//! doc comment.
//!
//! Scope: checks `pub fn` at the item level (free functions); the fuller
//! doctrine (`# Errors`/`# Panics` sections) is a style refinement this
//! rule does not yet enforce structurally — presence of any `///` line
//! above the item is what this rule checks.

use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Attribute, ItemFn, Visibility};

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// The `RUST-DOC-PUBLIC-ITEM` `Validator`.
pub struct DocPublicItemValidator {
    rule_id: RuleId,
}

impl DocPublicItemValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: "RUST-DOC-PUBLIC-ITEM".parse()?,
        })
    }
}

impl Validator for DocPublicItemValidator {
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

fn has_doc_comment(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("doc"))
}

fn is_public(vis: &Visibility) -> bool {
    matches!(vis, Visibility::Public(_))
}

struct Visitor {
    rule_id: RuleId,
    file: RelPath,
    findings: Vec<Finding>,
}

impl<'ast> Visit<'ast> for Visitor {
    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        if is_public(&item.vis) && !has_doc_comment(&item.attrs) {
            let line = u32::try_from(item.span().start().line.max(1)).unwrap_or(u32::MAX);
            self.findings.push(Finding {
                rule_id: self.rule_id.clone(),
                severity: Severity::Warning,
                title: format!("`pub fn {}` has no `///` doc comment", item.sig.ident),
                detail: format!(
                    "Fix: add a `///` summary above `pub fn {}` (and `# Errors`/`# Panics` \
                     sections if it returns `Result` or can panic).",
                    item.sig.ident
                ),
                file: self.file.clone(),
                line,
                snippet: None,
            });
        }
        visit::visit_item_fn(self, item);
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;

    use super::DocPublicItemValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn fires_on_missing_doc_and_silent_when_documented() -> Result<(), Box<dyn std::error::Error>> {
        let validator = DocPublicItemValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "fixtures/doc-public-item/fail_no_doc.rs",
            "fixtures/doc-public-item/pass_documented.rs",
        )?;
        Ok(())
    }

    #[test]
    fn unparseable_source_stays_silent() -> Result<(), Box<dyn std::error::Error>> {
        use enforcer_validator::validator::Validator;
        let validator = DocPublicItemValidator::new()?;
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
