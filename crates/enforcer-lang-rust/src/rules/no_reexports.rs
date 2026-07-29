//! `T1-NOREEXPORT.1` — no re-export barrels.
//!
//! Adapted from OcentraParent's `no-reexports` println-style AST check into
//! a structured [`Validator`]: bans `pub use` / `pub(crate) use` barrels
//! (import concrete module paths instead) and rejects the
//! `const _ = size_of::<T>()` keep-alive idiom some barrel modules use to
//! silence "unused import" warnings while still re-exporting.
//!
//! Keyed to arc-04's rule record
//! (`crates/enforcer-rules/rules/no-reexports.json`, `ruleId`
//! `T1-NOREEXPORT.1`). `use` statements with NO visibility modifier
//! (private `use`) are unaffected — this rule targets barrels that
//! RE-EXPORT, not ordinary internal imports.

use syn::visit::{self, Visit};
use syn::{Item, ItemConst, ItemUse, Visibility};

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::{BuiltInRustRule, RuleId};
use enforcer_domain::rules_types::RulePredicateResult;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// The `T1-NOREEXPORT.1` no-reexports `Validator`.
#[derive(Debug)]
pub struct NoReexportsValidator {
    rule_id: RuleId,
}

impl NoReexportsValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary; the literal below is proven valid
    /// by `enforcer_domain::ids` tests, and this constructor is exercised
    /// by this module's own tests).
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: BuiltInRustRule::NoReexports.id(),
        })
    }
}

impl Validator for NoReexportsValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Ok(file) = syn::parse_file(input.source.as_str()) else {
            // Unparseable source is not this validator's concern (a syntax
            // check belongs elsewhere); stay silent rather than guess.
            return Vec::new();
        };

        let mut visitor = ReexportVisitor {
            rule_id: &self.rule_id,
            file: input.file,
            findings: Vec::new(),
        };
        visitor.visit_file(&file);
        visitor.findings
    }
}

struct ReexportVisitor<'a> {
    rule_id: &'a RuleId,
    file: &'a enforcer_domain::paths::RelPath,
    findings: Vec<Finding>,
}

impl ReexportVisitor<'_> {
    fn push(
        &mut self,
        line: enforcer_domain::telemetry_types::SourceLine,
        title: &str,
        detail: &str,
    ) {
        let Ok(finding) = crate::boundary::finding::from_source(
            (self.rule_id, Severity::Error),
            title,
            detail,
            self.file,
            line,
        ) else {
            return;
        };
        self.findings.push(finding);
    }
}

/// A `pub`/`pub(crate)` `use` is a barrel re-export; a bare private `use`
/// is an ordinary import and is not flagged.
fn is_reexporting_visibility(vis: &Visibility) -> RulePredicateResult {
    if matches!(vis, Visibility::Public(_) | Visibility::Restricted(_)) {
        RulePredicateResult::Matched
    } else {
        RulePredicateResult::NotMatched
    }
}

impl<'ast> Visit<'ast> for ReexportVisitor<'_> {
    fn visit_item_use(&mut self, item: &'ast ItemUse) {
        if is_reexporting_visibility(&item.vis) == RulePredicateResult::Matched {
            let line = crate::boundary::finding::source_line(item);
            self.push(
                line,
                "no re-export barrels (pub use / pub(crate) use)",
                "Fix: remove this re-export; have callers import the concrete module path \
                 directly instead of re-exporting it through a barrel.",
            );
        }
        visit::visit_item_use(self, item);
    }

    fn visit_item_const(&mut self, item: &'ast ItemConst) {
        if is_size_of_keep_alive(item) == RulePredicateResult::Matched {
            let line = crate::boundary::finding::source_line(item);
            self.push(
                line,
                "rejected `const _ = size_of` keep-alive idiom",
                "Fix: delete this keep-alive const; it exists only to silence an \"unused \
                 import\" warning on a re-export this rule already bans. Remove the \
                 re-export instead of keeping it alive.",
            );
        }
        visit::visit_item_const(self, item);
    }

    fn visit_item(&mut self, item: &'ast Item) {
        // Recurse into nested `mod` blocks (barrels are often one level
        // deep inside a `mod reexports { ... }`); the default visit walk
        // already does this via `visit_item_mod`, so just delegate.
        visit::visit_item(self, item);
    }
}

/// Detects the `const _ = size_of::<T>()` / `size_of_val(...)` idiom: an
/// underscore-named const binding whose initializer calls `size_of`-family
/// functions, used to keep an otherwise-unused re-exported type "alive" for
/// the compiler without a real use site.
fn is_size_of_keep_alive(item: &ItemConst) -> RulePredicateResult {
    if item.ident != "_" {
        return RulePredicateResult::NotMatched;
    }
    expr_calls_size_of(&item.expr)
}

fn expr_calls_size_of(expr: &syn::Expr) -> RulePredicateResult {
    match expr {
        syn::Expr::Call(call) => path_is_size_of(&call.func),
        syn::Expr::MethodCall(call) if call.method == "size_of" || call.method == "size_of_val" => {
            RulePredicateResult::Matched
        }
        _ => RulePredicateResult::NotMatched,
    }
}

fn path_is_size_of(expr: &syn::Expr) -> RulePredicateResult {
    let syn::Expr::Path(path_expr) = expr else {
        return RulePredicateResult::NotMatched;
    };
    if path_expr
        .path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "size_of" || segment.ident == "size_of_val")
    {
        RulePredicateResult::Matched
    } else {
        RulePredicateResult::NotMatched
    }
}

#[cfg(test)]
mod tests {

    use crate::boundary::fixture::run_fixture_parity;
    use enforcer_validator::validator::Validator;

    use super::NoReexportsValidator;

    #[test]
    fn fires_on_pub_use_barrel_and_silent_on_direct_paths() -> Result<(), Box<dyn std::error::Error>>
    {
        let validator = NoReexportsValidator::new()?;
        run_fixture_parity(
            &validator,
            "fixtures/no-reexports/fail_barrel.rs",
            "fixtures/no-reexports/pass_direct_paths.rs",
        )?;
        Ok(())
    }

    #[test]
    fn size_of_keep_alive_idiom_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let validator = NoReexportsValidator::new()?;
        let source = "pub(crate) use crate::inner::Thing;\nconst _: () = size_of::<Thing>();\n";
        let file: enforcer_domain::paths::RelPath =
            crate::boundary::fixture::source_file("crates/x/src/lib.rs")?;
        let findings = validator.validate(enforcer_validator::validator::ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(source),
            scope: enforcer_domain::findings::ScanScope::Files,
        });
        // Both the barrel `use` AND the keep-alive const must be flagged.
        assert_eq!(findings.len(), 2);
        assert!(findings
            .iter()
            .all(|f| f.rule_id.as_str() == "T1-NOREEXPORT.1"));
        Ok(())
    }

    #[test]
    fn private_use_is_not_flagged() -> Result<(), Box<dyn std::error::Error>> {
        let validator = NoReexportsValidator::new()?;
        let source = "use crate::inner::Thing;\n";
        let file: enforcer_domain::paths::RelPath =
            crate::boundary::fixture::source_file("crates/x/src/lib.rs")?;
        let findings = validator.validate(enforcer_validator::validator::ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(source),
            scope: enforcer_domain::findings::ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }

    #[test]
    fn malformed_source_stays_silent() -> Result<(), Box<dyn std::error::Error>> {
        let validator = NoReexportsValidator::new()?;
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
