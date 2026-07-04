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

use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Item, ItemConst, ItemUse, Visibility};

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// The `T1-NOREEXPORT.1` no-reexports `Validator`.
pub struct NoReexportsValidator {
    rule_id: RuleId,
}

impl NoReexportsValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary; the literal below is proven valid
    /// by `enforcer_domain::ids` tests, and this constructor is exercised
    /// by this module's own tests).
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: "T1-NOREEXPORT.1".parse()?,
        })
    }
}

impl Validator for NoReexportsValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Ok(file) = syn::parse_file(input.source) else {
            // Unparseable source is not this validator's concern (a syntax
            // check belongs elsewhere); stay silent rather than guess.
            return Vec::new();
        };

        let mut visitor = ReexportVisitor {
            rule_id: self.rule_id.clone(),
            file: input.file.clone(),
            findings: Vec::new(),
        };
        visitor.visit_file(&file);
        visitor.findings
    }
}

struct ReexportVisitor {
    rule_id: RuleId,
    file: enforcer_domain::paths::RelPath,
    findings: Vec<Finding>,
}

impl ReexportVisitor {
    fn line_of<S: Spanned>(&self, spanned: &S) -> u32 {
        let line = spanned.span().start().line;
        // proc-macro2 line numbers are already 1-based; guard against a 0
        // from a degenerate span rather than emitting a nonsensical 0.
        if line == 0 {
            1
        } else {
            u32::try_from(line).unwrap_or(u32::MAX)
        }
    }

    fn push(&mut self, line: u32, title: &str, detail: String) {
        self.findings.push(Finding {
            rule_id: self.rule_id.clone(),
            severity: Severity::Error,
            title: title.to_owned(),
            detail,
            file: self.file.clone(),
            line,
            snippet: None,
        });
    }
}

/// A `pub`/`pub(crate)` `use` is a barrel re-export; a bare private `use`
/// is an ordinary import and is not flagged.
fn is_reexporting_visibility(vis: &Visibility) -> bool {
    matches!(vis, Visibility::Public(_) | Visibility::Restricted(_))
}

impl<'ast> Visit<'ast> for ReexportVisitor {
    fn visit_item_use(&mut self, item: &'ast ItemUse) {
        if is_reexporting_visibility(&item.vis) {
            let line = self.line_of(item);
            self.push(
                line,
                "no re-export barrels (pub use / pub(crate) use)",
                "Fix: remove this re-export; have callers import the concrete module path \
                 directly instead of re-exporting it through a barrel."
                    .to_owned(),
            );
        }
        visit::visit_item_use(self, item);
    }

    fn visit_item_const(&mut self, item: &'ast ItemConst) {
        if is_size_of_keep_alive(item) {
            let line = self.line_of(item);
            self.push(
                line,
                "rejected `const _ = size_of` keep-alive idiom",
                "Fix: delete this keep-alive const; it exists only to silence an \"unused \
                 import\" warning on a re-export this rule already bans. Remove the \
                 re-export instead of keeping it alive."
                    .to_owned(),
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
fn is_size_of_keep_alive(item: &ItemConst) -> bool {
    if item.ident != "_" {
        return false;
    }
    expr_calls_size_of(&item.expr)
}

fn expr_calls_size_of(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::Call(call) => path_is_size_of(&call.func),
        syn::Expr::MethodCall(call) => call.method == "size_of" || call.method == "size_of_val",
        _ => false,
    }
}

fn path_is_size_of(expr: &syn::Expr) -> bool {
    let syn::Expr::Path(path_expr) = expr else {
        return false;
    };
    path_expr
        .path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "size_of" || segment.ident == "size_of_val")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;
    use enforcer_validator::validator::Validator;

    use super::NoReexportsValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn fires_on_pub_use_barrel_and_silent_on_direct_paths() -> Result<(), Box<dyn std::error::Error>>
    {
        let validator = NoReexportsValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "fixtures/no-reexports/fail_barrel.rs",
            "fixtures/no-reexports/pass_direct_paths.rs",
        )?;
        Ok(())
    }

    #[test]
    fn size_of_keep_alive_idiom_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let validator = NoReexportsValidator::new()?;
        let source = "pub(crate) use crate::inner::Thing;\nconst _: () = size_of::<Thing>();\n";
        let file: enforcer_domain::paths::RelPath = "crates/x/src/lib.rs".parse()?;
        let findings = validator.validate(enforcer_validator::validator::ValidationInput {
            file: &file,
            source,
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
        let file: enforcer_domain::paths::RelPath = "crates/x/src/lib.rs".parse()?;
        let findings = validator.validate(enforcer_validator::validator::ValidationInput {
            file: &file,
            source,
            scope: enforcer_domain::findings::ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }

    #[test]
    fn unparseable_source_stays_silent() -> Result<(), Box<dyn std::error::Error>> {
        let validator = NoReexportsValidator::new()?;
        let file: enforcer_domain::paths::RelPath = "crates/x/src/lib.rs".parse()?;
        let findings = validator.validate(enforcer_validator::validator::ValidationInput {
            file: &file,
            source: "this is not valid rust {{{",
            scope: enforcer_domain::findings::ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }
}
