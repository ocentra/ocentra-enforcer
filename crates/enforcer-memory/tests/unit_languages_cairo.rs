//! Hard tests for Cairo, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_cairo`])
//! -- there is no bespoke `languages::cairo` extractor to prove
//! zero-regression against (Cairo has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::cairo`]'s own doc
//! comment directly: positional `function_definition` naming (no `name`
//! field), the real `struct_item`/`mod_item` name fields, and
//! `call_expression`'s by-kind (not by-field) argument-list lookup.

use enforcer_memory::languages::generic::parse_cairo;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_cairo";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_function_definition_via_positional_name() {
    let src = "fn helper(x: u128) -> u128 {\n    x + 1\n}\n";
    let parsed = parse_cairo(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_struct_item_via_real_name_field() {
    let src = "struct Storage {\n    value: u128,\n}\n";
    let parsed = parse_cairo(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Storage"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_mod_item_via_real_name_field() {
    let src = "mod Counter {\n    fn helper() {}\n}\n";
    let parsed = parse_cairo(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Counter"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_call_with_multiple_args_via_unwrapped_positional_children() -> TestResult {
    // Regression guard for the confirmed no-wrapping-node call-argument
    // shape (see `LangSpec::cairo`'s own doc comment): without
    // `cairo_call_override`/`cairo_call_arg_texts` walking
    // `call_expression`'s own direct children, `arg_texts` would come
    // back empty (there is no separate `arguments` node nested inside
    // `call_expression` at all for this grammar's own call shape).
    let src = "fn main() {\n    helper(1, 2, 3);\n}\n";
    let parsed = parse_cairo(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(
        call.arg_texts,
        vec!["1".to_string(), "2".to_string(), "3".to_string()],
        "{call:?}"
    );
    Ok(())
}

#[test]
fn extracts_call_inside_function_with_from_symbol_scope() -> TestResult {
    let src = "fn main() {\n    helper();\n}\n";
    let parsed = parse_cairo(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("main"));
    Ok(())
}

#[test]
fn extracts_use_declaration_as_import() -> TestResult {
    let src = "use starknet::ContractAddress;\n";
    let parsed = parse_cairo(src);
    assert!(!parsed.imports.is_empty(), "{:?}", parsed.imports);
    Ok(())
}

#[test]
fn extracts_branch_heavy_function_without_panicking() {
    let src = "fn helper(x: u128) -> u128 {\n    if x > 0 {\n        x + 1\n    } else {\n        0\n    }\n}\n";
    let parsed = parse_cairo(src);
    assert!(symbol_kind(&parsed.symbols, "helper").is_some());
}

#[test]
fn parses_fixture_counter_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("counter.cairo");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_cairo(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Counter"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Storage"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "increment"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert!(!parsed.imports.is_empty(), "{:?}", parsed.imports);
    Ok(())
}

#[test]
fn attributed_function_and_struct_record_decorates_edges() -> TestResult {
    // language-parity wave G3 stage 3: Cairo's `#[...]` `attribute_item`
    // node has no fields at all -- its name is a positional
    // `identifier`/`scoped_identifier` child -- and is simply the
    // PREVIOUS SIBLING of the function/struct it decorates, not a field
    // or wrapper (identical shape to `tree-sitter-rust`'s own
    // `attribute_item`).
    let src = r#"
#[external(v0)]
fn draw() {}

#[derive(Drop)]
struct Widget {}
"#;
    let parsed = parse_cairo(src);
    let fn_edge = parsed
        .decorates
        .iter()
        .find(|d| d.target_name == "draw")
        .ok_or("expected a DECORATES edge for draw")?;
    assert_eq!(fn_edge.decorator_name, "external");
    let struct_edge = parsed
        .decorates
        .iter()
        .find(|d| d.target_name == "Widget")
        .ok_or("expected a DECORATES edge for Widget")?;
    assert_eq!(struct_edge.decorator_name, "derive");
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "fn main() {\n    helper();\n}\n";
    let first = parse_cairo(src);
    let second = parse_cairo(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_cairo("fn ( { this is not valid cairo @@@");
    let _ = parsed;
}
