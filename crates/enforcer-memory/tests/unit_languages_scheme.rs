//! Hard tests for Scheme, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_memory::languages::generic::parse_scheme`]) -- there is
//! no bespoke `languages::scheme` extractor to prove zero-regression
//! against (Scheme has never had one in this crate), so these tests
//! assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::scheme`]'s own doc
//! comment directly: def-form recognition and disambiguation from a
//! plain call (both are `list` nodes with no syntactic distinction), the
//! baseline's own def-head keyword table narrowed to Scheme's real
//! subset (`define`/`define-record-type`/...), the baseline's
//! UNFILTERED call-callee recording (a def-form's own head keyword is
//! ALSO recorded as a call -- intentional, matches
//! [`enforcer_memory::languages::generic::clojure_quirks`]'s identical
//! posture), and `import`/`require`/`load`/`include` IMPORTS.

use enforcer_memory::languages::generic::parse_scheme;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_scheme";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn define_is_a_function_symbol() {
    let src = "(define (greet name) (display name))\n";
    let parsed = parse_scheme(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "greet"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn define_record_type_is_a_struct_symbol() {
    let src = "(define-record-type point (make-point x y) point? (x point-x) (y point-y))\n";
    let parsed = parse_scheme(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "point"),
        Some(&SymbolKind::Struct),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn def_form_head_is_also_recorded_as_a_call() {
    let src = "(define (greet name) (display name))\n";
    let parsed = parse_scheme(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"define"), "{callees:?}");
}

#[test]
fn call_inside_define_body_records_from_symbol_scope() -> TestResult {
    let src = "(define (greet name) (draw name))\n";
    let parsed = parse_scheme(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "draw")
        .ok_or("expected a draw call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("greet"), "{call:?}");
    Ok(())
}

#[test]
fn module_scope_call_has_no_from_symbol() -> TestResult {
    let src = "(greet \"world\")\n";
    let parsed = parse_scheme(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "greet")
        .ok_or("expected a greet call")?;
    assert_eq!(call.from_symbol, None, "{call:?}");
    Ok(())
}

#[test]
fn extracts_require_as_import() {
    let src = "(require scheme/string)\n";
    let parsed = parse_scheme(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"scheme/string"), "{paths:?}");
}

#[test]
fn extracts_import_of_list_spec_as_import() {
    let src = "(import (scheme base))\n";
    let parsed = parse_scheme(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"scheme base"), "{paths:?}");
}

#[test]
fn ordinary_call_is_not_misdetected_as_an_import() {
    let src = "(greet \"world\")\n";
    let parsed = parse_scheme(src);
    assert!(parsed.imports.is_empty(), "{:?}", parsed.imports);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_scheme("(define ( this is not valid scheme @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.scm");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_scheme(&src);
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
