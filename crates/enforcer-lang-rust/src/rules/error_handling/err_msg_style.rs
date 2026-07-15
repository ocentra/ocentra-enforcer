//! `RUST-ERR-MSG-STYLE` (T2) — `#[error("...")]` message strings
//! (`thiserror`) should be lowercase-led and free of trailing punctuation,
//! matching Rust's std-error message convention.

use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Attribute, Lit, Meta};

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// The `RUST-ERR-MSG-STYLE` `Validator`.
pub struct ErrMsgStyleValidator {
    rule_id: RuleId,
}

impl ErrMsgStyleValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: "RUST-ERR-MSG-STYLE".parse()?,
        })
    }
}

impl Validator for ErrMsgStyleValidator {
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

/// Extract the leading string literal out of `#[error("...")]`, ignoring
/// any `{field}` format arguments that follow it in the token stream —
/// `syn`'s `Meta::List` for `#[error(...)]` carries the whole token stream
/// as opaque tokens (thiserror is a derive macro, not a builtin attribute
/// `syn` parses structurally), so this re-parses the first token as a
/// string literal.
fn error_message_literal(attr: &Attribute) -> Option<String> {
    if !attr.path().is_ident("error") {
        return None;
    }
    let Meta::List(list) = &attr.meta else {
        return None;
    };
    let lit: Lit = syn::parse2(list.tokens.clone()).ok()?;
    match lit {
        Lit::Str(lit_str) => Some(lit_str.value()),
        _ => None,
    }
}

fn style_violation(message: &str) -> Option<&'static str> {
    if message.is_empty() {
        return None;
    }
    if message.ends_with('.') || message.ends_with('!') {
        return Some("ends with trailing punctuation");
    }
    if let Some(first) = message.chars().next() {
        if first.is_uppercase() {
            return Some("starts with an uppercase letter");
        }
    }
    None
}

struct Visitor {
    rule_id: RuleId,
    file: RelPath,
    findings: Vec<Finding>,
}

impl<'ast> Visit<'ast> for Visitor {
    fn visit_attribute(&mut self, attr: &'ast Attribute) {
        if let Some(message) = error_message_literal(attr) {
            if let Some(reason) = style_violation(&message) {
                let line = u32::try_from(attr.span().start().line.max(1)).unwrap_or(u32::MAX);
                self.findings.push(Finding {
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Warning,
                    title: format!("error message style: {reason}"),
                    detail: format!(
                        "Fix: rewrite \"{message}\" to start lowercase and drop trailing \
                         punctuation, matching Rust's std-error message convention."
                    ),
                    file: self.file.clone(),
                    line,
                    snippet: None,
                });
            }
        }
        visit::visit_attribute(self, attr);
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;

    use super::ErrMsgStyleValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn fires_on_bad_style_and_silent_on_good_style() -> Result<(), Box<dyn std::error::Error>> {
        let validator = ErrMsgStyleValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "fixtures/err-msg-style/fail_style.rs",
            "fixtures/err-msg-style/pass_style.rs",
        )?;
        Ok(())
    }

    #[test]
    fn unparseable_source_stays_silent() -> Result<(), Box<dyn std::error::Error>> {
        use enforcer_validator::validator::Validator;
        let validator = ErrMsgStyleValidator::new()?;
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
