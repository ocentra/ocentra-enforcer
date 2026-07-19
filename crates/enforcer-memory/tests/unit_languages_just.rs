//! Hard tests for Just, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_just`])
//! -- there is no bespoke `languages::just` extractor to prove
//! zero-regression against (Just has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::just`]'s own doc comment
//! directly: `recipe`'s nested `recipe_header`/`recipe_body` naming,
//! `function_call`'s real `"name"`/`"arguments"` fields, and
//! `dependency`'s own bare-name CALLS-edge convention.

use enforcer_memory::languages::generic::parse_just;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_just";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_recipe_as_function_symbol() {
    let src = "build:\n    cargo build\n";
    let parsed = parse_just(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "build"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_dependency_as_call_edge() -> TestResult {
    let src = "build: setup\n    cargo build\n\nsetup:\n    echo setup\n";
    let parsed = parse_just(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "setup")
        .ok_or("expected a setup dependency call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("build"));
    Ok(())
}

#[test]
fn extracts_function_call_with_arguments() -> TestResult {
    let src = "greet:\n    echo {{trim(\"hi\")}}\n";
    let parsed = parse_just(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "trim")
        .ok_or("expected a trim function call")?;
    assert!(!call.arg_texts.is_empty(), "{call:?}");
    Ok(())
}

#[test]
fn extracts_import_as_import_edge() {
    let src = "import 'common.just'\n\nbuild:\n    cargo build\n";
    let parsed = parse_just(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"common.just"), "{paths:?}");
}

#[test]
fn parses_fixture_widget_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("widget.just");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_just(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "build"),
        Some(&SymbolKind::Function)
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "setup"),
        Some(&SymbolKind::Function)
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "test"),
        Some(&SymbolKind::Function)
    );
    assert!(
        parsed.calls.iter().any(|c| c.callee == "setup"),
        "{:?}",
        parsed.calls
    );
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "build: setup\n    cargo build\n\nsetup:\n    echo setup\n";
    let first = parse_just(src);
    let second = parse_just(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_just("recipe ( { this is not valid just @@@");
    let _ = parsed;
}
