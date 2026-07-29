//! Hard tests for Puppet, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_puppet`])
//! -- there is no bespoke `languages::puppet` extractor to prove
//! zero-regression against (Puppet has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::puppet`]'s own doc
//! comment directly: this grammar is entirely field-free for definitions,
//! `defined_resource_type` (the real, confirmed baseline gap this row
//! fills), `node_definition`'s own `node_name` child, and
//! `include_statement`'s dual CALLS+IMPORTS recording off one node visit.

use enforcer_memory::languages::generic::parse_puppet;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_puppet";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_class_definition_as_class() {
    let src = "class widget {\n  $x = 1\n}\n";
    let parsed = parse_puppet(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "widget"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_defined_resource_type_as_class() {
    // `defined_resource_type` (Puppet's `define` keyword) is a real,
    // confirmed baseline gap this row fills -- see `LangSpec::puppet`'s
    // own doc comment.
    let src = "define helper($x) {\n  notify { \"hi\": }\n}\n";
    let parsed = parse_puppet(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_function_declaration_as_function() {
    let src = "function widget::double(Integer $x) {\n  $x\n}\n";
    let parsed = parse_puppet(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "widget::double"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_node_definition_name_off_node_name_child() {
    let src = "node 'web01.example.com' {\n  include widget\n}\n";
    let parsed = parse_puppet(src);
    assert!(
        parsed
            .symbols
            .iter()
            .any(|s| s.name.contains("web01.example.com")),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn include_statement_records_both_call_and_import() -> TestResult {
    let src = "include stdlib\n";
    let parsed = parse_puppet(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "include")
        .ok_or("expected an include call")?;
    let _ = call;
    let import = parsed
        .imports
        .iter()
        .find(|i| i.module_path == "stdlib")
        .ok_or("expected an stdlib import")?;
    let _ = import;
    Ok(())
}

#[test]
fn call_inside_class_body_is_found_via_unfielded_block_recursion() -> TestResult {
    let src = "class widget {\n  helper($x)\n}\n";
    let parsed = parse_puppet(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a nested helper call inside the class body")?;
    assert_eq!(call.from_symbol.as_deref(), Some("widget"), "{call:?}");
    Ok(())
}

#[test]
fn resource_declaration_records_type_as_both_symbol_and_call() -> TestResult {
    let src = "notify { \"found\": }\n";
    let parsed = parse_puppet(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "notify"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "notify")
        .ok_or("expected a notify call")?;
    let _ = call;
    Ok(())
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_puppet("class ??? this is not valid puppet @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.pp");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_puppet(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "widget"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "helper"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.imports.iter().any(|i| i.module_path == "stdlib"),
        "{:?}",
        parsed.imports
    );
    Ok(())
}
