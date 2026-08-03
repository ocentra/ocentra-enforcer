//! Hard tests for Pine (TradingView Pine Script), onboarded directly
//! through the generic spec-table engine
//! ([`enforcer_syntax::languages::generic::parse_pine`]) -- grammar
//! VENDORED (`vendor/tree-sitter-pine-local/`). Asserts against the
//! grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::pine`]'s own doc
//! comment: `function_declaration_statement`'s fieldless-for-`"name"`
//! name resolution via its own `function`/`method` fields (claimed by
//! [`enforcer_syntax::languages::generic::pine_quirk`]), and real
//! `type_definition_statement`/`call` fields.

use enforcer_syntax::languages::generic::parse_pine;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_pine";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_function_declaration_via_quirk() {
    let src = "myFunc(x, y) =>\n    x + y\n";
    let parsed = parse_pine(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "myFunc"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_type_definition_via_real_name_field() {
    let src = "type Point\n    float x\n    float y\n";
    let parsed = parse_pine(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Point"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_call_inside_function_with_from_symbol_scope() -> TestResult {
    let src = "myFunc(x, y) =>\n    plotResult(x + y)\n";
    let parsed = parse_pine(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "plotResult")
        .ok_or("expected a plotResult call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("myFunc"));
    Ok(())
}

#[test]
fn extracts_top_level_call_via_real_fields() -> TestResult {
    let src = "plot(1)\n";
    let parsed = parse_pine(src);
    parsed
        .calls
        .iter()
        .find(|c| c.callee == "plot")
        .ok_or("expected a plot call")?;
    Ok(())
}

#[test]
fn parses_fixture_strategy_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("strategy.pine");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_pine(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "myFunc"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.calls.iter().any(|c| c.callee == "plot"),
        "{:?}",
        parsed.calls
    );
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "myFunc(x, y) =>\n    x + y\n";
    let first = parse_pine(src);
    let second = parse_pine(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_pine("this is not valid pine @@@ ===> <<<");
    let _ = parsed;
}
