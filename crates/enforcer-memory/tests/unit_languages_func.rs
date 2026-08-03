//! Hard tests for FunC, onboarded directly through the generic
//! spec-table engine ([`enforcer_syntax::languages::generic::parse_func`])
//! -- there is no bespoke `languages::func` extractor to prove
//! zero-regression against (FunC has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::func`]'s own doc
//! comment directly: the real `function_definition` name field, the
//! `function_application`/`method_call` confirmed grammar TYPO
//! (`"agruments"`, not `"arguments"`) this crate's own
//! `func_call_override` must read correctly regardless, and the
//! confirmed-unextractable `#include` (parses as a bare `ERROR` node in
//! this grammar, not a spec-writing gap).

use enforcer_syntax::languages::generic::parse_func;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_func";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_function_definition_via_real_name_field() {
    let src = "int add(int a, int b) {\n    return a + b;\n}\n";
    let parsed = parse_func(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "add"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_function_application_call_despite_agruments_typo() -> TestResult {
    // Regression guard for the confirmed grammar typo (see
    // `LangSpec::func`'s own doc comment): without `func_call_override`
    // reading the misspelled `"agruments"` field explicitly, `arg_texts`
    // would come back empty (the generic default's `call_arguments_field`
    // lookup for the correctly-spelled `"arguments"` finds nothing on this
    // node kind).
    let src = "() main() {\n    int x = add(1, 2);\n}\n";
    let parsed = parse_func(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "add")
        .ok_or("expected an add call")?;
    assert_eq!(
        call.arg_texts,
        vec!["1".to_string(), "2".to_string()],
        "{call:?}"
    );
    Ok(())
}

#[test]
fn extracts_method_call_via_tilde_syntax() -> TestResult {
    let src = "() main() {\n    int x = 5;\n    x~dump();\n}\n";
    let parsed = parse_func(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "dump")
        .ok_or("expected a dump method call")?;
    let _ = call;
    Ok(())
}

#[test]
fn extracts_method_call_args_via_tensor_expression_wrapper() -> TestResult {
    // Regression guard for the SAME wrapper-unwrapping fix
    // `extracts_function_application_call_despite_agruments_typo` covers
    // for `function_application` -- `method_call`'s own `"arguments"`
    // field has the identical three-shape-by-argument-count wrapping
    // (see `func_call_arg_texts`'s own doc comment).
    let src = "() main() {\n    int x = 5;\n    x~add(1, 2);\n}\n";
    let parsed = parse_func(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "add")
        .ok_or("expected an add method call")?;
    assert_eq!(
        call.arg_texts,
        vec!["1".to_string(), "2".to_string()],
        "{call:?}"
    );
    Ok(())
}

#[test]
fn extracts_call_with_from_symbol_scope() -> TestResult {
    let src = "() main() {\n    int x = add(1, 2);\n}\n";
    let parsed = parse_func(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "add")
        .ok_or("expected an add call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("main"));
    Ok(())
}

#[test]
fn include_directive_produces_no_import_edge() {
    // Confirmed grammar limitation, not a spec-writing gap -- see
    // `LangSpec::func`'s own doc comment: `#include "stdlib.fc";` parses
    // as a bare `ERROR` node in this grammar version, so there is no real
    // node this crate could ever extract an IMPORTS edge from. This test
    // documents/guards the current (empty) behavior rather than a lost
    // extraction opportunity.
    let src = "#include \"stdlib.fc\";\n\nint add(int a, int b) {\n    return a + b;\n}\n";
    let parsed = parse_func(src);
    assert!(parsed.imports.is_empty(), "{:?}", parsed.imports);
    // The function definition after the (unparseable) include directive
    // must still extract cleanly.
    assert_eq!(
        symbol_kind(&parsed.symbols, "add"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn parses_fixture_counter_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("counter.fc");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_func(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "add"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "main"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "add")
        .ok_or("expected an add call")?;
    assert_eq!(
        call.arg_texts,
        vec!["1".to_string(), "2".to_string()],
        "{call:?}"
    );
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "int add(int a, int b) {\n    return a + b;\n}\n";
    let first = parse_func(src);
    let second = parse_func(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_func("int ( { this is not valid func @@@");
    let _ = parsed;
}
