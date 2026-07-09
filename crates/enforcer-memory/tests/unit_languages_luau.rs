//! Hard tests for Luau, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_memory::languages::generic::parse_luau`]) -- there is no
//! bespoke `languages::luau` extractor to prove zero-regression against,
//! so these tests assert against the grammar-shape ground truth recorded
//! in [`enforcer_memory::languages::spec::LangSpec::luau`]'s own doc
//! comment directly: plain/dotted `function_declaration` naming, the
//! anonymous `function_definition` literal, `type_definition` type
//! aliases, ordinary calls, and branch recognition.

use enforcer_memory::languages::generic::parse_luau;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_luau";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_free_function_declaration() {
    let src = r#"
local function foo(a: number, b: number): number
	return a + b
end
"#;
    let parsed = parse_luau(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "foo"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_dotted_function_declaration_with_qualified_name() {
    let src = r#"
function t.foo(a)
	return a
end
"#;
    let parsed = parse_luau(src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "t.foo"),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_type_definition() {
    let src = r#"type Point = { x: number, y: number }"#;
    let parsed = parse_luau(src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Point"),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn anonymous_function_definition_does_not_panic() {
    let src = r#"
local foo = function(a)
	return a
end
"#;
    let parsed = parse_luau(src);
    let _ = parsed;
}

#[test]
fn extracts_plain_call_with_from_symbol_scope() -> TestResult {
    let src = r#"
function render(w)
	helper(1, 2)
end
"#;
    let parsed = parse_luau(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("render"), "{call:?}");
    assert_eq!(
        call.arg_texts,
        vec!["1".to_string(), "2".to_string()],
        "{call:?}"
    );
    Ok(())
}

#[test]
fn extracts_calls_inside_if_branch() {
    let src = r#"
function render(w)
	if w then
		helper()
	end
end
"#;
    let parsed = parse_luau(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"helper"), "{callees:?}");
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_luau("function ( { this is not valid luau @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.luau");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_luau(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "helper"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Widget.new"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Point"),
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
