//! Hard tests for Typst, onboarded directly through the generic
//! spec-table engine ([`enforcer_syntax::languages::generic::parse_typst`])
//! -- there is no bespoke `languages::typst` extractor to prove
//! zero-regression against (Typst has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::typst`]'s own doc comment
//! directly: `let`'s conditional function-vs-variable naming (only a
//! function when its own `pattern` field is itself a nested `call` node),
//! `call`'s single-`item`-field full claim (no separate `arguments` field
//! exists at all), and `import`/`include`'s positional path IMPORTS.

use enforcer_syntax::languages::generic::parse_typst;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_typst";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_let_with_call_pattern_as_function() {
    let src = r#"
#let helper(x) = x + 1
"#;
    let parsed = parse_typst(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn plain_let_binding_is_not_treated_as_a_function() {
    // Regression guard for the baseline's own documented behavior
    // (`extract_defs.c:935`): a non-call `pattern` resolves to no name at
    // all, "keeping value bindings out of func_types".
    let src = r#"
#let x = 1
"#;
    let parsed = parse_typst(src);
    assert!(
        !parsed.symbols.iter().any(|s| s.name == "x"),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_import_as_imports_edge() {
    let src = r#"
#import "widget-lib.typ": draw_helper
"#;
    let parsed = parse_typst(src);
    assert!(!parsed.imports.is_empty(), "{:?}", parsed.imports);
}

#[test]
fn extracts_function_call_via_item_field() -> TestResult {
    let src = r#"
#let draw(x) = {
  helper(x)
}
"#;
    let parsed = parse_typst(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee.starts_with("helper"))
        .ok_or("expected a helper call")?;
    let _ = call;
    Ok(())
}

#[test]
fn call_inside_function_records_from_symbol_scope() -> TestResult {
    let src = r#"
#let draw(x) = {
  helper(x)
}
"#;
    let parsed = parse_typst(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee.starts_with("helper"))
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("draw"), "{call:?}");
    Ok(())
}

#[test]
fn if_expression_is_recognized_as_a_branch_node() {
    let src = r#"
#let draw(x) = {
  if x == "" {
    helper(x)
  } else {
    helper(x)
  }
}
"#;
    let parsed = parse_typst(src);
    let helper_calls = parsed
        .calls
        .iter()
        .filter(|c| c.callee.starts_with("helper"))
        .count();
    assert_eq!(helper_calls, 2, "{:?}", parsed.calls);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_typst("#let ( { this is not valid typst @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.typ");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_typst(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "helper"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "draw"),
        "{:?}",
        parsed.symbols
    );
    assert!(!parsed.imports.is_empty(), "{:?}", parsed.imports);
    Ok(())
}
