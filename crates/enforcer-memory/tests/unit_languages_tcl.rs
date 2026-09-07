//! Hard tests for Tcl, onboarded directly through the generic
//! spec-table engine ([`enforcer_syntax::languages::generic::parse_tcl`])
//! -- there is no bespoke `languages::tcl` extractor to prove
//! zero-regression against (Tcl has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::tcl`]'s own doc comment
//! directly: `procedure`'s real `name`/`body` field pair (handled
//! natively), `command` CALLS with `arguments`-field `arg_texts`
//! (handled natively), and `namespace eval NAME {...}` as a Class
//! symbol scoping nested `proc`s.

use enforcer_syntax::languages::generic::parse_tcl;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_tcl";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_procedure_definition() {
    let src = "proc greet {name} {\n    puts $name\n}\n";
    let parsed = parse_tcl(src);
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
    let parsed = parse_tcl(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "greet")
        .ok_or("expected a greet call")?;
    assert_eq!(call.arg_texts, vec!["world".to_string()], "{call:?}");
    Ok(())
}

#[test]
fn call_inside_procedure_records_from_symbol_scope() -> TestResult {
    let src = "proc render {} {\n    greet world\n}\n";
    let parsed = parse_tcl(src);
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
    let parsed = parse_tcl(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "greet")
        .ok_or("expected a greet call")?;
    assert_eq!(call.from_symbol, None, "{call:?}");
    Ok(())
}

#[test]
fn namespace_eval_is_a_class_symbol() {
    let src = "namespace eval Widgets {\n    proc make {} {\n        puts hi\n    }\n}\n";
    let parsed = parse_tcl(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Widgets"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn incomplete_namespace_eval_is_not_classified_or_panics() {
    let parsed = parse_tcl("namespace eval\n");
    assert!(
        !parsed.symbols.iter().any(|symbol| symbol.name == "eval"),
        "{:#?}",
        parsed.symbols
    );
}

#[test]
fn proc_nested_in_namespace_is_defines_attributed() -> TestResult {
    let src = "namespace eval Widgets {\n    proc make {} {\n        puts hi\n    }\n}\n";
    let parsed = parse_tcl(src);
    let def = parsed
        .defines
        .iter()
        .find(|d| d.member_name == "make")
        .ok_or("expected a make DEFINES edge")?;
    assert_eq!(def.container_name, "Widgets", "{def:?}");
    Ok(())
}

#[test]
fn extracts_calls_inside_if_branches() {
    let src = "proc render {} {\n    if {1} {\n        greet yes\n    } else {\n        greet no\n    }\n}\n";
    let parsed = parse_tcl(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert_eq!(callees.iter().filter(|c| **c == "greet").count(), 2);
}

#[test]
fn no_imports_are_ever_recorded() {
    let src = "proc greet {name} {\n    puts $name\n}\ngreet world\n";
    let parsed = parse_tcl(src);
    assert!(parsed.imports.is_empty(), "{:?}", parsed.imports);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_tcl("proc ( { this is not valid tcl @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.tcl");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_tcl(&src);
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
        parsed.symbols.iter().any(|s| s.name == "Widgets"),
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
