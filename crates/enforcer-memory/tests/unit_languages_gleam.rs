//! Hard tests for Gleam, onboarded directly through the generic
//! spec-table engine ([`enforcer_syntax::languages::generic::parse_gleam`])
//! -- there is no bespoke `languages::gleam` extractor to prove
//! zero-regression against (Gleam has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::gleam`]'s own doc
//! comment directly: `function`/`external_function` ordinary name-field
//! naming (no quirk needed), `type_definition`/`type_alias` positional
//! `type_name` naming, `import`'s `module`-field IMPORTS, and
//! `function_call`'s ordinary `function`/`arguments` fields (the only
//! one of this wave's three languages needing zero call-override quirk
//! at all).

use enforcer_syntax::languages::generic::parse_gleam;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_gleam";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_public_function() {
    let src = r#"
pub fn helper(x: Int) -> Int {
  x + 1
}
"#;
    let parsed = parse_gleam(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_custom_type_as_class() {
    // Gleam's "custom type" declaration is grammatically just
    // `type_definition` -- the baseline's own `custom_type` array entry
    // is a phantom node kind this grammar never generates (see
    // `LangSpec::gleam`'s own doc comment).
    let src = r#"
pub type Shape {
  Circle(radius: Float)
  Square(side: Float)
}
"#;
    let parsed = parse_gleam(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Shape"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_type_alias() {
    let src = r#"
pub type Meters =
  Float
"#;
    let parsed = parse_gleam(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Meters"),
        Some(&SymbolKind::TypeAlias),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_import_module_field_as_imports_edge() {
    let src = r#"
import gleam/io
"#;
    let parsed = parse_gleam(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"gleam/io"));
}

#[test]
fn extracts_function_call_with_real_fields() -> TestResult {
    let src = r#"
pub fn draw(x: Int) {
  helper(x)
}
"#;
    let parsed = parse_gleam(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.arg_texts, vec!["x".to_string()], "{call:?}");
    Ok(())
}

#[test]
fn extracts_field_access_receiver_call() -> TestResult {
    let src = r#"
pub fn draw(x: Int) {
  io.println("drawing")
}
"#;
    let parsed = parse_gleam(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "io.println")
        .ok_or("expected an io.println call")?;
    let _ = call;
    Ok(())
}

#[test]
fn call_inside_function_records_from_symbol_scope() -> TestResult {
    let src = r#"
pub fn draw(x: Int) {
  helper(x)
}
"#;
    let parsed = parse_gleam(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("draw"), "{call:?}");
    Ok(())
}

#[test]
fn case_expression_is_recognized_as_a_branch_node() {
    // No direct public API surfaces branch counting from this test
    // module, so this only asserts the fixture with a `case` inside a
    // function body still parses cleanly end-to-end (branch_types feeds
    // complexity computation elsewhere, not `ParsedFile` directly).
    let src = r#"
pub fn draw(x: Int) {
  case x {
    0 -> helper(x)
    _ -> helper(x)
  }
}
"#;
    let parsed = parse_gleam(src);
    let helper_calls = parsed.calls.iter().filter(|c| c.callee == "helper").count();
    assert_eq!(helper_calls, 2, "{:?}", parsed.calls);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_gleam("pub fn ( { this is not valid gleam @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.gleam");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_gleam(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Shape"),
        "{:?}",
        parsed.symbols
    );
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
        parsed.imports.iter().any(|i| i.module_path == "gleam/io"),
        "{:?}",
        parsed.imports
    );
    Ok(())
}
