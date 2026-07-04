//! `T1-RUSTERR.1` — d17 rust-error-handling.
//!
//! Flags the `unwrap`/`expect`/`panic!`/`todo!`/`unimplemented!`/`dbg!`
//! family in first-party (non-`#[cfg(test)]`) Rust source. This
//! complements the workspace's `[workspace.lints.clippy]` deny-wall
//! (`unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`,
//! `dbg_macro` — a01) with a structured, doc-anchored, fixable
//! [`Finding`] for CONSUMER repos that have not opted into that deny-wall
//! themselves: clippy only enforces this inside THIS workspace, this
//! `Validator` enforces the same discipline as an enforcer rule any
//! consumer repo can run.
//!
//! `#[cfg(test)]` modules/functions and anything inside a
//! `#[test]`-attributed function are exempt — test code legitimately uses
//! `unwrap`/`expect`/`panic!` for assertions, matching d17's proof-row
//! contract ("unwrap under `#[cfg(test)]` stays clean").

use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Attribute, ExprMethodCall, ImplItemFn, Item, ItemFn, ItemMod, TraitItemFn};

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

// Each sibling rule lives in its own submodule (not re-exported — this
// crate's own `no_reexports` rule bans `pub use` barrels, so callers import
// e.g. `error_handling::nonexhaustive::NonExhaustiveValidator` directly).
pub mod allow_reason;
pub mod arch_main_thin;
pub mod borrow_param;
pub mod cast_lossy;
pub mod doc_public_item;
pub mod err_context;
pub mod err_main_exitcode;
pub mod err_msg_style;
pub mod err_sentinel;
pub mod fmt_captured_ident;
pub mod fn_complexity;
pub mod fn_max_params;
pub mod layer_domain;
pub mod match_wildcard;
pub mod mcp_stdout;
pub mod no_utils_module;
pub mod nonexhaustive;
pub mod safety_comment;

/// The `T1-RUSTERR.1` d17 rust-error-handling `Validator`.
pub struct ErrorHandlingValidator {
    rule_id: RuleId,
}

impl ErrorHandlingValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary, mirroring
    /// [`super::no_reexports::NoReexportsValidator::new`]).
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: "T1-RUSTERR.1".parse()?,
        })
    }
}

impl Validator for ErrorHandlingValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Ok(file) = syn::parse_file(input.source) else {
            return Vec::new();
        };

        let mut visitor = ErrorHandlingVisitor {
            rule_id: self.rule_id.clone(),
            file: input.file.clone(),
            findings: Vec::new(),
            test_depth: 0,
        };
        visitor.visit_file(&file);
        visitor.findings
    }
}

/// Method names on `Result`/`Option` this rule bans outside test code.
const BANNED_METHODS: &[&str] = &["unwrap", "expect"];

/// Macro names (by their final path segment) this rule bans outside test
/// code.
const BANNED_MACROS: &[&str] = &["panic", "todo", "unimplemented", "dbg"];

struct ErrorHandlingVisitor {
    rule_id: RuleId,
    file: RelPath,
    findings: Vec<Finding>,
    /// Depth of nesting inside a `#[cfg(test)]` module or `#[test]`
    /// function. Non-zero means the current position is exempt.
    test_depth: usize,
}

impl ErrorHandlingVisitor {
    fn line_of<S: Spanned>(&self, spanned: &S) -> u32 {
        let line = spanned.span().start().line;
        if line == 0 {
            1
        } else {
            u32::try_from(line).unwrap_or(u32::MAX)
        }
    }

    fn push(&mut self, line: u32, marker: &str) {
        self.findings.push(Finding {
            rule_id: self.rule_id.clone(),
            severity: Severity::Error,
            title: format!("rust-error-handling: `{marker}` in first-party code"),
            detail: format!(
                "Fix: replace `{marker}` with a typed `Result`/`Option` propagation \
                 (`?` + `.with_context(...)`, or a matched fallback); `{marker}` is only \
                 permitted under `#[cfg(test)]` / `#[test]`."
            ),
            file: self.file.clone(),
            line,
            snippet: None,
        });
    }

    fn enter_test_scope(&mut self, is_test: bool, body: impl FnOnce(&mut Self)) {
        if is_test {
            self.test_depth += 1;
        }
        body(self);
        if is_test {
            self.test_depth -= 1;
        }
    }
}

/// True when the attribute list carries `#[cfg(test)]` or `#[test]`.
fn attrs_mark_test(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if attr.path().is_ident("test") {
            return true;
        }
        if attr.path().is_ident("cfg") {
            // `#[cfg(test)]` — inspect the meta list for the literal
            // identifier `test`.
            let mut found = false;
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("test") {
                    found = true;
                }
                Ok(())
            });
            return found;
        }
        false
    })
}

impl<'ast> Visit<'ast> for ErrorHandlingVisitor {
    fn visit_item_mod(&mut self, item: &'ast ItemMod) {
        let is_test = attrs_mark_test(&item.attrs);
        self.enter_test_scope(is_test, |v| visit::visit_item_mod(v, item));
    }

    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        let is_test = attrs_mark_test(&item.attrs);
        self.enter_test_scope(is_test, |v| visit::visit_item_fn(v, item));
    }

    fn visit_impl_item_fn(&mut self, item: &'ast ImplItemFn) {
        let is_test = attrs_mark_test(&item.attrs);
        self.enter_test_scope(is_test, |v| visit::visit_impl_item_fn(v, item));
    }

    fn visit_trait_item_fn(&mut self, item: &'ast TraitItemFn) {
        let is_test = attrs_mark_test(&item.attrs);
        self.enter_test_scope(is_test, |v| visit::visit_trait_item_fn(v, item));
    }

    fn visit_item(&mut self, item: &'ast Item) {
        visit::visit_item(self, item);
    }

    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        if self.test_depth == 0 {
            let method = call.method.to_string();
            if BANNED_METHODS.contains(&method.as_str()) {
                let line = self.line_of(call);
                self.push(line, &method);
            }
        }
        visit::visit_expr_method_call(self, call);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        // `syn`'s default walk calls `visit_macro` for every macro
        // invocation regardless of position (expression, statement, or
        // item context) — covering `visit_expr_macro` SEPARATELY would
        // double-count `dbg!(x)` (visited as both an `ExprMacro` node and
        // the `Macro` it wraps), so this is the single point of truth.
        if self.test_depth == 0 {
            if let Some(segment) = mac.path.segments.last() {
                let name = segment.ident.to_string();
                if BANNED_MACROS.contains(&name.as_str()) {
                    let line = self.line_of(mac);
                    self.push(line, &name);
                }
            }
        }
        visit::visit_macro(self, mac);
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;
    use enforcer_validator::validator::Validator;

    use super::ErrorHandlingValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn fires_on_unwrap_in_first_party_and_silent_on_result_propagation(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = ErrorHandlingValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "fixtures/error-handling/fail_unwrap.rs",
            "fixtures/error-handling/pass_result_propagation.rs",
        )?;
        Ok(())
    }

    #[test]
    fn unwrap_under_cfg_test_module_stays_clean() -> Result<(), Box<dyn std::error::Error>> {
        let validator = ErrorHandlingValidator::new()?;
        let source = "fn producer() -> Option<i32> { Some(1) }\n\
                       #[cfg(test)]\n\
                       mod tests {\n\
                       #[test]\n\
                       fn it_works() {\n\
                       let v = super::producer().unwrap();\n\
                       assert_eq!(v, 1);\n\
                       }\n\
                       }\n";
        let file: enforcer_domain::paths::RelPath = "crates/x/src/lib.rs".parse()?;
        let findings = validator.validate(enforcer_validator::validator::ValidationInput {
            file: &file,
            source,
            scope: enforcer_domain::findings::ScanScope::Files,
        });
        assert!(
            findings.is_empty(),
            "unwrap under #[cfg(test)] must stay clean"
        );
        Ok(())
    }

    #[test]
    fn panic_todo_unimplemented_dbg_all_fire_in_first_party_code(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = ErrorHandlingValidator::new()?;
        let source = "fn a() { panic!(\"x\"); }\n\
                       fn b() { todo!(); }\n\
                       fn c() { unimplemented!(); }\n\
                       fn d(x: i32) -> i32 { dbg!(x) }\n";
        let file: enforcer_domain::paths::RelPath = "crates/x/src/lib.rs".parse()?;
        let findings = validator.validate(enforcer_validator::validator::ValidationInput {
            file: &file,
            source,
            scope: enforcer_domain::findings::ScanScope::Files,
        });
        assert_eq!(findings.len(), 4);
        assert!(findings
            .iter()
            .all(|f| f.rule_id.as_str() == "T1-RUSTERR.1"));
        Ok(())
    }

    #[test]
    fn unparseable_source_stays_silent() -> Result<(), Box<dyn std::error::Error>> {
        let validator = ErrorHandlingValidator::new()?;
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
