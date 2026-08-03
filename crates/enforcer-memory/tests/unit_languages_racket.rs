//! Hard tests for Racket, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_syntax::languages::generic::parse_racket`]) -- there is
//! no bespoke `languages::racket` extractor to prove zero-regression
//! against (Racket has never had one in this crate), so these tests
//! assert against the grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::racket`]'s own doc
//! comment directly: def-form recognition (including `(struct ...)` as
//! a Struct symbol, via the SAME `list`-based head-keyword quirk -- NOT
//! the grammar's unrelated `structure` node kind, which is `#s(...)`
//! prefab-literal syntax, see that const's own doc comment), the
//! baseline's UNFILTERED call-callee recording, and `require` IMPORTS.

use enforcer_syntax::languages::generic::parse_racket;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_racket";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn define_is_a_function_symbol() {
    let src = "(define (greet name) (display name))\n";
    let parsed = parse_racket(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "greet"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn struct_form_is_a_struct_symbol() {
    let src = "(struct point (x y))\n";
    let parsed = parse_racket(src);
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
    let parsed = parse_racket(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"define"));
}

#[test]
fn call_inside_define_body_records_from_symbol_scope() -> TestResult {
    let src = "(define (greet name) (draw name))\n";
    let parsed = parse_racket(src);
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
    let parsed = parse_racket(src);
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
    let src = "(require racket/string)\n";
    let parsed = parse_racket(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"racket/string"));
}

#[test]
fn ordinary_call_is_not_misdetected_as_an_import() {
    let src = "(greet \"world\")\n";
    let parsed = parse_racket(src);
    assert!(parsed.imports.is_empty(), "{:?}", parsed.imports);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_racket("(define ( this is not valid racket @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.rkt");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_racket(&src);
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
        symbol_kind(&parsed.symbols, "point") == Some(&SymbolKind::Struct),
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
