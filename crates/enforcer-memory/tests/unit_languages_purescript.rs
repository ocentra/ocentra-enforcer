//! Hard tests for PureScript, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_syntax::languages::generic::parse_purescript`]) -- there
//! is no bespoke `languages::purescript` extractor to prove
//! zero-regression against (PureScript has never had one in this crate),
//! so these tests assert against the grammar-shape ground truth recorded
//! in
//! [`enforcer_syntax::languages::spec::LangSpec::purescript`]'s own doc
//! comment directly: `function`'s own `"rhs"`-not-`"body"` field plus
//! multi-child `"name"` filtering, `class_declaration`'s nested
//! `class_head`/`class_name` naming, and `exp_apply`'s fieldless
//! callee-plus-positional-arguments shape.

use enforcer_syntax::languages::generic::parse_purescript;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_purescript";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_function_symbol_via_rhs_field() {
    let src = "add :: Int -> Int -> Int\nadd a b = a + b\n";
    let parsed = parse_purescript(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "add"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_data_declaration_as_class_symbol() {
    let src = "data Color = Red | Green | Blue\n";
    let parsed = parse_purescript(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Color"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_class_declaration_via_nested_class_head_name() {
    let src = "class Shape a where\n  area :: a -> Number\n";
    let parsed = parse_purescript(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Shape"),
        Some(&SymbolKind::Interface),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_exp_apply_call_with_positional_arguments() -> TestResult {
    let src = "main = add 1 2\n";
    let parsed = parse_purescript(src);
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
fn extracts_import_with_module_field() {
    let src = "import Prelude\n";
    let parsed = parse_purescript(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"Prelude"));
}

#[test]
fn parses_fixture_widget_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("widget.purs");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_purescript(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "add"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Color"),
        Some(&SymbolKind::Class)
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Shape"),
        Some(&SymbolKind::Interface)
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "main"),
        Some(&SymbolKind::Function)
    );
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "main = add 1 2\n";
    let first = parse_purescript(src);
    let second = parse_purescript(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_purescript("module ( { this is not valid purescript @@@");
    let _ = parsed;
}
