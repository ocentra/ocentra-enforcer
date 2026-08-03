//! Hard tests for Emacs Lisp, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_syntax::languages::generic::parse_emacslisp`]). Asserts
//! against the grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::emacslisp`]'s own doc
//! comment: real `function_definition`/`macro_definition` `name`
//! fields, the missing-`body`-field closure via
//! [`enforcer_syntax::languages::generic::emacslisp_on_method_defined`],
//! and every fieldless `list` node's callee resolution via
//! [`enforcer_syntax::languages::generic::emacslisp_call_override`].

use enforcer_syntax::languages::generic::parse_emacslisp;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_emacslisp";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_defun_via_real_name_field() {
    let src = "(defun add-numbers (a b)\n  (+ a b))\n";
    let parsed = parse_emacslisp(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "add-numbers"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_defmacro_via_real_name_field() {
    let src = "(defmacro my-macro (x)\n  `(list ,x))\n";
    let parsed = parse_emacslisp(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "my-macro"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_call_inside_defun_body_with_from_symbol_scope() -> TestResult {
    // Regression guard for the confirmed missing-`body`-field finding
    // (see `LangSpec::emacslisp`'s own doc comment): without
    // `emacslisp_on_method_defined`, this call would never be visited at
    // all (the generic engine's own `body_field`-driven recursion finds
    // nothing to recurse into).
    let src = "(defun add-numbers (a b)\n  (+ a b))\n";
    let parsed = parse_emacslisp(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "+")
        .ok_or("expected a + call inside add-numbers' own body")?;
    assert_eq!(call.from_symbol.as_deref(), Some("add-numbers"));
    Ok(())
}

#[test]
fn does_not_misread_parameter_list_as_a_call() {
    // Regression guard: `(a b)` is the PARAMETER list, not a call to a
    // function literally named `a` -- `emacslisp_on_method_defined`
    // skips it by node identity.
    let src = "(defun add-numbers (a b)\n  1)\n";
    let parsed = parse_emacslisp(src);
    assert!(
        !parsed.calls.iter().any(|c| c.callee == "a"),
        "{:?}",
        parsed.calls
    );
}

#[test]
fn extracts_top_level_call_via_override() -> TestResult {
    let src = "(add-numbers 1 2)\n";
    let parsed = parse_emacslisp(src);
    parsed
        .calls
        .iter()
        .find(|c| c.callee == "add-numbers")
        .ok_or("expected an add-numbers call")?;
    Ok(())
}

#[test]
fn parses_fixture_utils_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("utils.el");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_emacslisp(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "add-numbers"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "my-macro"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.calls.iter().any(|c| c.callee == "add-numbers"),
        "{:?}",
        parsed.calls
    );
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "(defun add-numbers (a b)\n  (+ a b))\n";
    let first = parse_emacslisp(src);
    let second = parse_emacslisp(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_emacslisp("(this is not valid elisp @@@ (((");
    let _ = parsed;
}
