//! `RUST-ERR-MSG-STYLE` (T2) — `#[error("...")]` message strings
//! (`thiserror`) should be lowercase-led and free of trailing punctuation,
//! matching Rust's std-error message convention.

use syn::visit::{self, Visit};
use syn::{Attribute, Lit, Meta};

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::{BuiltInRustRule, RuleId};
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// The `RUST-ERR-MSG-STYLE` `Validator`.
#[derive(Debug)]
pub struct ErrMsgStyleValidator {
    rule_id: RuleId,
}

impl ErrMsgStyleValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: BuiltInRustRule::ErrMsgStyle.id(),
        })
    }
}

impl Validator for ErrMsgStyleValidator {
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
    let lit: Lit = match list.parse_args() {
        Ok(lit) => lit,
        Err(_) => return None,
    };
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

struct Visitor<'a> {
    rule_id: &'a RuleId,
    file: &'a RelPath,
    findings: Vec<Finding>,
}

impl<'ast> Visit<'ast> for Visitor<'_> {
    fn visit_attribute(&mut self, attr: &'ast Attribute) {
        if let Some(message) = error_message_literal(attr) {
            if let Some(reason) = style_violation(&message) {
                let line = crate::boundary::finding::source_line(attr);
                let Ok(finding) = crate::boundary::finding::from_source(
                    (self.rule_id, Severity::Warning),
                    format!("error message style: {reason}"),
                    format!(
                        "Fix: rewrite \"{message}\" to start lowercase and drop trailing \
                         punctuation, matching Rust's std-error message convention."
                    ),
                    self.file,
                    line,
                ) else {
                    return;
                };
                self.findings.push(finding);
            }
        }
        visit::visit_attribute(self, attr);
    }
}

#[cfg(test)]
mod tests {

    use crate::boundary::fixture::run_fixture_parity;

    use super::ErrMsgStyleValidator;

    #[test]
    fn fires_on_bad_style_and_silent_on_good_style() -> Result<(), Box<dyn std::error::Error>> {
        let validator = ErrMsgStyleValidator::new()?;
        run_fixture_parity(
            &validator,
            "fixtures/err-msg-style/fail_style.rs",
            "fixtures/err-msg-style/pass_style.rs",
        )?;
        Ok(())
    }

    #[test]
    fn malformed_source_stays_silent() -> Result<(), Box<dyn std::error::Error>> {
        use enforcer_validator::validator::Validator;
        let validator = ErrMsgStyleValidator::new()?;
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
