//! Hard tests for Teal, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_syntax::languages::generic::parse_teal`]) -- there is no
//! bespoke `languages::teal` extractor to prove zero-regression against,
//! so these tests assert against the grammar-shape ground truth recorded
//! in [`enforcer_syntax::languages::spec::LangSpec::teal`]'s own doc
//! comment directly: plain/dotted `function_statement` naming,
//! `record_declaration` DEFINES nesting, `function_call`'s own
//! `called_object` field, and branch recognition (including the
//! `numeric_for_statement` correction over baseline's bare
//! `for_statement`).

use enforcer_syntax::languages::generic::parse_teal;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_teal";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_free_function_statement() {
    let src = r#"
local function foo(a: number, b: number): number
   return a + b
end
"#;
    let parsed = parse_teal(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "foo"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_dotted_function_statement_with_qualified_name() {
    let src = r#"
function t.foo(a: number): number
   return a
end
"#;
    let parsed = parse_teal(src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "t.foo"),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_record_declaration() {
    let src = r#"
local record Widget
   name: string
end
"#;
    let parsed = parse_teal(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Widget"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_plain_call_with_from_symbol_scope() -> TestResult {
    let src = r#"
local function render(w: string)
   helper(w)
end
"#;
    let parsed = parse_teal(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("render"), "{call:?}");
    Ok(())
}

#[test]
fn extracts_calls_inside_numeric_for_loop() {
    let src = r#"
local function render()
   for i = 1, 10 do
      helper(i)
   end
end
"#;
    let parsed = parse_teal(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"helper"));
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_teal("function ( { this is not valid teal @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.tl");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_teal(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Widget"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Widget.new"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "helper"),
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
