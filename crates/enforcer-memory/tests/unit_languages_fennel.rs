//! Hard tests for Fennel, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_memory::languages::generic::parse_fennel`]) -- there is no
//! bespoke `languages::fennel` extractor to prove zero-regression
//! against, so these tests assert against the grammar-shape ground truth
//! recorded in [`enforcer_memory::languages::spec::LangSpec::fennel`]'s
//! own doc comment directly: `fn_form`'s quirk-claimed optional-name
//! handling, `list`'s own `call` field, and that a nested call inside an
//! anonymous `hashfn_form` is still found (the exact `body_field`-gap
//! scenario the quirk exists for).

use enforcer_memory::languages::generic::parse_fennel;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_fennel";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_named_fn_form() {
    let src = r#"(fn foo [a b] (+ a b))"#;
    let parsed = parse_fennel(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "foo"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_named_lambda_form() {
    let src = r#"(lambda foo [a] a)"#;
    let parsed = parse_fennel(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "foo"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn call_inside_named_fn_records_from_symbol_scope() -> TestResult {
    let src = r#"(fn foo [a] (helper a))"#;
    let parsed = parse_fennel(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("foo"), "{call:?}");
    Ok(())
}

#[test]
fn anonymous_hashfn_form_still_finds_nested_call() {
    // `hashfn_form` (`#(...)`) has NO `name` field at all -- this is the
    // exact `body_field`-gap scenario [`LangSpec::fennel`]'s own doc
    // comment documents: without the quirk's own unconditional-walk
    // fallback, this nested call would be silently dropped.
    let src = r#"(each [x (pairs t)] (#(helper x)))"#;
    let parsed = parse_fennel(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"helper"), "{callees:?}");
}

#[test]
fn extracts_calls_inside_each_form() {
    let src = r#"(each [k v (pairs t)] (print k v))"#;
    let parsed = parse_fennel(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"pairs"), "{callees:?}");
    assert!(callees.contains(&"print"), "{callees:?}");
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_fennel("(fn ( this is not valid fennel @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.fnl");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_fennel(&src);
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
        parsed.calls.iter().any(|c| c.callee == "helper"),
        "{:?}",
        parsed.calls
    );
    Ok(())
}
