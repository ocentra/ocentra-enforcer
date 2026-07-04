//! `RUST-FMT-CAPTURED-IDENT` (T2) — prefer inline captured format
//! identifiers (`format!("{path}")`) over positional/named args
//! (`format!("{}", path)`).
//!
//! Scope: flags a `format!`/`println!`/`eprintln!`/`write!`/`writeln!`
//! invocation whose format-string literal contains a bare positional `{}`
//! placeholder while the macro call has at least one trailing argument
//! after the format string — the common case an inline capture could
//! replace.

use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{ExprMacro, Lit};

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

const FMT_MACROS: &[&str] = &[
    "format", "println", "eprintln", "print", "eprint", "write", "writeln",
];

/// The `RUST-FMT-CAPTURED-IDENT` `Validator`.
pub struct FmtCapturedIdentValidator {
    rule_id: RuleId,
}

impl FmtCapturedIdentValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: "RUST-FMT-CAPTURED-IDENT".parse()?,
        })
    }
}

impl Validator for FmtCapturedIdentValidator {
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

/// Parses the macro's token stream as a comma-separated expression list,
/// as a `format!`-family macro invocation would be written; returns
/// `None` if the tokens do not parse this way (e.g. non-format macro
/// content matched by name coincidentally).
fn parse_macro_args(mac: &syn::Macro) -> Option<Vec<syn::Expr>> {
    let parser = syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated;
    let parsed = mac.parse_body_with(parser).ok()?;
    Some(parsed.into_iter().collect())
}

fn has_positional_placeholder_with_trailing_args(mac: &syn::Macro) -> bool {
    let Some(args) = parse_macro_args(mac) else {
        return false;
    };
    let Some(first) = args.first() else {
        return false;
    };
    let syn::Expr::Lit(expr_lit) = first else {
        return false;
    };
    let Lit::Str(lit_str) = &expr_lit.lit else {
        return false;
    };
    lit_str.value().contains("{}") && args.len() > 1
}

struct Visitor {
    rule_id: RuleId,
    file: RelPath,
    findings: Vec<Finding>,
}

impl<'ast> Visit<'ast> for Visitor {
    fn visit_expr_macro(&mut self, item: &'ast ExprMacro) {
        if let Some(segment) = item.mac.path.segments.last() {
            let name = segment.ident.to_string();
            if FMT_MACROS.contains(&name.as_str())
                && has_positional_placeholder_with_trailing_args(&item.mac)
            {
                let line = u32::try_from(item.span().start().line.max(1)).unwrap_or(u32::MAX);
                self.findings.push(Finding {
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Warning,
                    title: format!("`{name}!` uses positional `{{}}` instead of a captured ident"),
                    detail: format!(
                        "Fix: replace the positional `{{}}` placeholder and its trailing \
                         argument in this `{name}!` call with an inline captured identifier, \
                         e.g. `{name}!(\"{{path}}\")`."
                    ),
                    file: self.file.clone(),
                    line,
                    snippet: None,
                });
            }
        }
        visit::visit_expr_macro(self, item);
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;

    use super::FmtCapturedIdentValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn fires_on_positional_and_silent_on_captured() -> Result<(), Box<dyn std::error::Error>> {
        let validator = FmtCapturedIdentValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "fixtures/fmt-captured-ident/fail_positional.rs",
            "fixtures/fmt-captured-ident/pass_captured.rs",
        )?;
        Ok(())
    }

    #[test]
    fn unparseable_source_stays_silent() -> Result<(), Box<dyn std::error::Error>> {
        use enforcer_validator::validator::Validator;
        let validator = FmtCapturedIdentValidator::new()?;
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
