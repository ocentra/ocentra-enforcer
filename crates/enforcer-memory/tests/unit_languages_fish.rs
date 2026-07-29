//! Hard tests for Fish, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_fish`])
//! -- there is no bespoke `languages::fish` extractor to prove
//! zero-regression against (Fish has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::fish`]'s own doc comment
//! directly: `function_definition` naming + scoped body walk despite
//! having no `body`-named field, and `command` CALLS with
//! `argument`-field `arg_texts`.

use enforcer_memory::languages::generic::parse_fish;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_fish";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_function_definition() {
    let src = "function greet\n    echo hi\nend\n";
    let parsed = parse_fish(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "greet"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_call_with_arg_texts() -> TestResult {
    let src = "greet world\n";
    let parsed = parse_fish(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "greet")
        .ok_or("expected a greet call")?;
    assert_eq!(call.arg_texts, vec!["world".to_string()], "{call:?}");
    Ok(())
}

#[test]
fn call_with_multiple_arguments_records_arg_texts() -> TestResult {
    let src = "greet world again\n";
    let parsed = parse_fish(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "greet")
        .ok_or("expected a greet call")?;
    assert_eq!(
        call.arg_texts,
        vec!["world".to_string(), "again".to_string()],
        "{call:?}"
    );
    Ok(())
}

#[test]
fn call_inside_function_records_from_symbol_scope() -> TestResult {
    let src = "function render\n    greet world\nend\n";
    let parsed = parse_fish(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "greet")
        .ok_or("expected a greet call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("render"), "{call:?}");
    Ok(())
}

#[test]
fn module_scope_call_has_no_from_symbol() -> TestResult {
    let src = "greet world\n";
    let parsed = parse_fish(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "greet")
        .ok_or("expected a greet call")?;
    assert_eq!(call.from_symbol, None, "{call:?}");
    Ok(())
}

#[test]
fn extracts_calls_inside_if_branches() {
    let src = "if true\n    greet yes\nelse\n    greet no\nend\n";
    let parsed = parse_fish(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert_eq!(callees.iter().filter(|c| **c == "greet").count(), 2);
}

#[test]
fn no_imports_are_ever_recorded() {
    let src = "function greet\n    echo hi\nend\ngreet\n";
    let parsed = parse_fish(src);
    assert!(parsed.imports.is_empty(), "{:?}", parsed.imports);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_fish("function ( this is not valid fish @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.fish");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_fish(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "greet"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "draw"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.calls.iter().any(|c| c.callee == "draw"),
        "{:?}",
        parsed.calls
    );
    Ok(())
}
