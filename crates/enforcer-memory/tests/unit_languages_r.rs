//! Hard tests for R, onboarded directly through the generic spec-table
//! engine ([`enforcer_syntax::languages::generic::parse_r`]) -- there is
//! no bespoke `languages::r` extractor to prove zero-regression against
//! (R has never had one in this crate), so these tests assert against
//! the grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::r`]'s own doc comment
//! directly: `function_definition` naming resolved off the enclosing
//! `binary_operator` (NOT any field on the node itself -- see that doc
//! comment for why the node's own `name` field is a trap), `library`/
//! `require`/`requireNamespace`/`loadNamespace`/`source`/`box::use`
//! IMPORTS off an ordinary `call` node, and ordinary callee/branch/
//! from_symbol-scope extraction.

use enforcer_syntax::languages::generic::parse_r;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_r";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn arrow_assigned_function_is_a_function_symbol() {
    let src = "helper <- function(x, y) {\n  x + y\n}\n";
    let parsed = parse_r(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn equals_assigned_function_is_also_a_function_symbol() {
    let src = "f = function(x) {\n  print(x)\n}\n";
    let parsed = parse_r(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "f"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn nested_function_definitions_both_resolve_a_real_name() {
    // Regression guard for the exact bug this row's own quirk exists to
    // avoid: without it, both of these would be minted with the literal
    // name "function" (the node's own broken `name` field), not "outer"/
    // "inner".
    let src = "outer <- function() {\n  inner <- function() {\n    helper()\n  }\n  inner()\n}\n";
    let parsed = parse_r(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "outer"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "inner"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().all(|s| s.name != "function"),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn call_inside_function_records_from_symbol_scope() -> TestResult {
    let src = "outer <- function() {\n  inner <- function() {\n    helper()\n  }\n  inner()\n}\n";
    let parsed = parse_r(src);
    let helper_call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper() call")?;
    assert_eq!(
        helper_call.from_symbol.as_deref(),
        Some("inner"),
        "{helper_call:?}"
    );
    let inner_call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "inner")
        .ok_or("expected an inner() call")?;
    assert_eq!(
        inner_call.from_symbol.as_deref(),
        Some("outer"),
        "{inner_call:?}"
    );
    Ok(())
}

#[test]
fn module_scope_call_has_no_from_symbol() -> TestResult {
    let src = "helper()\n";
    let parsed = parse_r(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol, None, "{call:?}");
    Ok(())
}

#[test]
fn namespaced_call_keeps_full_callee_text() -> TestResult {
    let src = "stats::sd(x)\n";
    let parsed = parse_r(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "stats::sd")
        .ok_or("expected a stats::sd call")?;
    assert_eq!(call.arg_texts, vec!["x".to_string()], "{call:?}");
    Ok(())
}

#[test]
fn dollar_accessor_call_keeps_full_callee_text() -> TestResult {
    let src = "Widget$new()\n";
    let parsed = parse_r(src);
    assert!(
        parsed.calls.iter().any(|c| c.callee == "Widget$new"),
        "{:?}",
        parsed.calls
    );
    Ok(())
}

#[test]
fn extracts_library_call_as_import() {
    let src = "library(dplyr)\n";
    let parsed = parse_r(src);
    assert!(
        parsed.imports.iter().any(|i| i.module_path == "dplyr"),
        "{:?}",
        parsed.imports
    );
}

#[test]
fn library_call_is_also_recorded_as_a_call() {
    let src = "library(dplyr)\n";
    let parsed = parse_r(src);
    assert!(
        parsed.calls.iter().any(|c| c.callee == "library"),
        "{:?}",
        parsed.calls
    );
}

#[test]
fn extracts_require_and_source_calls_as_imports() {
    let src = "require(methods)\nsource(\"helper.R\")\n";
    let parsed = parse_r(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"methods"));
    assert!(paths.contains(&"helper.R"));
}

#[test]
fn extracts_box_use_call_as_import() {
    let src = "box::use(pkg/mod)\n";
    let parsed = parse_r(src);
    assert!(
        parsed.imports.iter().any(|i| i.module_path == "pkg/mod"),
        "{:?}",
        parsed.imports
    );
}

#[test]
fn ordinary_call_is_not_misdetected_as_an_import() {
    let src = "helper(1, 2)\n";
    let parsed = parse_r(src);
    assert!(parsed.imports.is_empty(), "{:?}", parsed.imports);
}

#[test]
fn extracts_if_for_while_as_branches() {
    // Branch counting itself lives in `crate::complexity`, not this
    // extractor -- this test only asserts the extractor does not choke on
    // (and correctly recurses through) every one of `LangSpec::r`'s own
    // `branch_types` shapes, still finding the calls nested inside each.
    let src = r#"
f <- function(x) {
  if (x > 0) {
    print("pos")
  }
  for (i in 1:10) {
    print(i)
  }
  while (x > 0) {
    x <- x - 1
  }
}
"#;
    let parsed = parse_r(src);
    let print_calls = parsed.calls.iter().filter(|c| c.callee == "print").count();
    assert!(print_calls >= 2, "{:?}", parsed.calls);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_r("f <- function( { this is not valid R @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.r");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_r(&src);
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
    assert!(
        parsed.imports.iter().any(|i| i.module_path == "methods"),
        "{:?}",
        parsed.imports
    );
    assert!(
        parsed.calls.iter().any(|c| c.callee == "helper"),
        "{:?}",
        parsed.calls
    );
    Ok(())
}
