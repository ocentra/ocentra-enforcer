//! `RUST-FMT-CAPTURED-IDENT` (T2) — prefer inline captured format
//! identifiers (`format!("{path}")`) over positional/named args
//! (`format!("{}", path)`).
//!
//! Scope: flags a `format!`/`println!`/`eprintln!`/`write!`/`writeln!`
//! invocation whose format-string literal contains a bare positional `{}`
//! placeholder while the macro call has at least one trailing argument
//! after the format string — the common case an inline capture could
//! replace.

use syn::visit::{self, Visit};
use syn::{ExprMacro, Lit};

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::{BuiltInRustRule, RuleId};
use enforcer_domain::paths::RelPath;
use enforcer_domain::rules_types::RulePredicateResult;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

const FMT_MACROS: &[&str] = &[
    "format", "println", "eprintln", "print", "eprint", "write", "writeln",
];

/// The `RUST-FMT-CAPTURED-IDENT` `Validator`.
#[derive(Debug)]
pub struct FmtCapturedIdentValidator {
    rule_id: RuleId,
}

impl FmtCapturedIdentValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: BuiltInRustRule::FmtCapturedIdent.id(),
        })
    }
}

impl Validator for FmtCapturedIdentValidator {
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

/// Parses the macro's token stream as a comma-separated expression list,
/// as a `format!`-family macro invocation would be written; returns
/// `None` if the tokens do not parse this way (e.g. non-format macro
/// content matched by name coincidentally).
fn parse_macro_args(mac: &syn::Macro) -> Option<Vec<syn::Expr>> {
    let parser = syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated;
    let parsed = match mac.parse_body_with(parser) {
        Ok(parsed) => parsed,
        Err(_) => return None,
    };
    Some(parsed.into_iter().collect())
}

fn has_positional_placeholder_with_trailing_args(mac: &syn::Macro) -> RulePredicateResult {
    let Some(args) = parse_macro_args(mac) else {
        return RulePredicateResult::NotMatched;
    };
    let Some(first) = args.first() else {
        return RulePredicateResult::NotMatched;
    };
    let syn::Expr::Lit(expr_lit) = first else {
        return RulePredicateResult::NotMatched;
    };
    let Lit::Str(lit_str) = &expr_lit.lit else {
        return RulePredicateResult::NotMatched;
    };
    if lit_str.value().contains("{}") && args.len() > 1 {
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
    fn visit_expr_macro(&mut self, item: &'ast ExprMacro) {
        if let Some(segment) = item.mac.path.segments.last() {
            let Some(name) = FMT_MACROS.iter().find(|name| segment.ident == **name) else {
                visit::visit_expr_macro(self, item);
                return;
            };
            if has_positional_placeholder_with_trailing_args(&item.mac)
                == RulePredicateResult::Matched
            {
                let line = crate::boundary::finding::source_line(item);
                let Ok(finding) = crate::boundary::finding::from_source(
                    (self.rule_id, Severity::Warning),
                    format!("`{name}!` uses positional `{{}}` instead of a captured ident"),
                    format!(
                        "Fix: replace the positional `{{}}` placeholder and its trailing \
                         argument in this `{name}!` call with an inline captured identifier, \
                         e.g. `{name}!(\"{{path}}\")`."
                    ),
                    self.file,
                    line,
                ) else {
                    return;
                };
                self.findings.push(finding);
            }
        }
        visit::visit_expr_macro(self, item);
    }
}

#[cfg(test)]
mod tests {

    use crate::boundary::fixture::run_fixture_parity;

    use super::{parse_macro_args, FmtCapturedIdentValidator};

    fn parse_macro(source: &str) -> Result<syn::Macro, syn::Error> {
        syn::parse_str::<syn::ExprMacro>(source).map(|expression| expression.mac)
    }

    #[test]
    fn fires_on_positional_and_silent_on_captured() -> Result<(), Box<dyn std::error::Error>> {
        let validator = FmtCapturedIdentValidator::new()?;
        run_fixture_parity(
            &validator,
            "fixtures/fmt-captured-ident/fail_positional.rs",
            "fixtures/fmt-captured-ident/pass_captured.rs",
        )?;
        Ok(())
    }

    #[test]
    fn malformed_source_stays_silent() -> Result<(), Box<dyn std::error::Error>> {
        use enforcer_validator::validator::Validator;
        let validator = FmtCapturedIdentValidator::new()?;
        let file: enforcer_domain::paths::RelPath =
            crate::boundary::fixture::source_file("crates/x/src/lib.rs")?;
        let findings = validator.validate(enforcer_validator::validator::ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(
                "not valid rust {{{",
            ),
            scope: enforcer_domain::findings::ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }

    #[test]
    fn parse_macro_args_rejects_malformed_tokens() -> Result<(), Box<dyn std::error::Error>> {
        let malformed = "format!(;)";
        let macro_call = parse_macro(malformed)?;
        assert!(parse_macro_args(&macro_call).is_none());
        Ok(())
    }

    #[test]
    fn parse_macro_args_handles_empty_input() -> Result<(), Box<dyn std::error::Error>> {
        let macro_call = parse_macro("format!()")?;
        assert_eq!(
            parse_macro_args(&macro_call).map(|args| args.len()),
            Some(0)
        );
        Ok(())
    }

    #[test]
    fn parse_macro_args_handles_oversized_argument_list() -> Result<(), Box<dyn std::error::Error>>
    {
        let arguments = std::iter::repeat_n("value", 128)
            .collect::<Vec<_>>()
            .join(",");
        let macro_call = parse_macro(&format!("format!({arguments})"))?;
        assert_eq!(
            parse_macro_args(&macro_call).map(|args| args.len()),
            Some(128)
        );
        Ok(())
    }

    #[test]
    fn parse_macro_args_rejects_invalid_non_expression_input(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let macro_call = parse_macro("format!(let value)")?;
        assert!(parse_macro_args(&macro_call).is_none());
        Ok(())
    }
}
