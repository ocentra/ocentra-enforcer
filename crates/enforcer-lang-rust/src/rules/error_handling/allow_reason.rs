//! `RUST-ALLOW-1.1` — every `#[allow(...)]`/`#[expect(...)]` attribute must
//! carry a `reason = "..."` key so a silenced lint is self-documenting
//! instead of a bare, unexplained suppression.

use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::Attribute;

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// The `RUST-ALLOW-1.1` `Validator`.
pub struct AllowReasonValidator {
    rule_id: RuleId,
}

impl AllowReasonValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: "RUST-ALLOW-1.1".parse()?,
        })
    }
}

impl Validator for AllowReasonValidator {
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

fn attr_name(attr: &Attribute) -> Option<&'static str> {
    if attr.path().is_ident("allow") {
        Some("allow")
    } else if attr.path().is_ident("expect") {
        Some("expect")
    } else {
        None
    }
}

fn has_reason(attr: &Attribute) -> bool {
    let mut found = false;
    let _ = attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("reason") {
            found = true;
        }
        // Consume the rest of this meta item (e.g. `= "..."`), ignoring
        // its value — presence of the `reason` key is all this rule
        // checks. `parse_nested_meta` requires the closure to consume any
        // associated value tokens itself.
        if meta.input.peek(syn::Token![=]) {
            let _: syn::Expr = meta.value()?.parse()?;
        }
        Ok(())
    });
    found
}

struct Visitor {
    rule_id: RuleId,
    file: RelPath,
    findings: Vec<Finding>,
}

impl Visitor {
    fn check_attrs(&mut self, attrs: &[Attribute]) {
        for attr in attrs {
            let Some(name) = attr_name(attr) else {
                continue;
            };
            if !has_reason(attr) {
                let line = u32::try_from(attr.span().start().line.max(1)).unwrap_or(u32::MAX);
                self.findings.push(Finding {
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Error,
                    title: format!("`#[{name}(...)]` with no `reason = \"...\"`"),
                    detail: format!(
                        "Fix: add a `reason = \"...\"` key inside this `#[{name}(...)]` \
                         explaining why the lint is suppressed."
                    ),
                    file: self.file.clone(),
                    line,
                    snippet: None,
                });
            }
        }
    }
}

impl<'ast> Visit<'ast> for Visitor {
    fn visit_attribute(&mut self, attr: &'ast Attribute) {
        self.check_attrs(std::slice::from_ref(attr));
        visit::visit_attribute(self, attr);
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;

    use super::AllowReasonValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn fires_on_missing_reason_and_silent_when_present() -> Result<(), Box<dyn std::error::Error>> {
        let validator = AllowReasonValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "fixtures/allow-reason/fail_no_reason.rs",
            "fixtures/allow-reason/pass_with_reason.rs",
        )?;
        Ok(())
    }

    #[test]
    fn unparseable_source_stays_silent() -> Result<(), Box<dyn std::error::Error>> {
        use enforcer_validator::validator::Validator;
        let validator = AllowReasonValidator::new()?;
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
