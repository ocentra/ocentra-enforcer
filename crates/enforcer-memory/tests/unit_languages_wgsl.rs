//! Hard tests for WGSL, onboarded directly through the generic
//! spec-table engine ([`enforcer_syntax::languages::generic::parse_wgsl`])
//! -- there is no bespoke `languages::wgsl` extractor to prove
//! zero-regression against (WGSL has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::wgsl`]'s own doc comment
//! directly: `function_declaration`/`struct_declaration`'s ordinary
//! `name` fields, `struct_declaration`'s Struct-kind relabeling,
//! `type_constructor_or_function_call_expression`'s zero-field
//! deepest-identifier-descent callee reconstruction, and
//! `enable_directive`'s positional-child IMPORTS.

use enforcer_syntax::languages::generic::parse_wgsl;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_wgsl";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_function_declaration() {
    let src = r#"
fn helper(x: f32) -> f32 {
    return x;
}
"#;
    let parsed = parse_wgsl(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_struct_declaration_as_struct_kind() {
    let src = r#"
struct Widget {
    label: f32,
};
"#;
    let parsed = parse_wgsl(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Widget"),
        Some(&SymbolKind::Struct),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_enable_directive_as_imports_edge() {
    let src = r#"
enable f16;
"#;
    let parsed = parse_wgsl(src);
    assert!(
        parsed.imports.iter().any(|i| i.module_path == "f16"),
        "{:?}",
        parsed.imports
    );
}

#[test]
fn extracts_function_call_via_deepest_identifier_descent() -> TestResult {
    let src = r#"
fn draw(x: f32) -> f32 {
    return helper(x);
}
"#;
    let parsed = parse_wgsl(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("draw"), "{call:?}");
    Ok(())
}

#[test]
fn if_statement_is_recognized_as_a_branch_node() {
    let src = r#"
fn draw(x: f32) -> f32 {
    if (x > 0.0) {
        return helper(x);
    }
    return helper(x);
}
"#;
    let parsed = parse_wgsl(src);
    let helper_calls = parsed.calls.iter().filter(|c| c.callee == "helper").count();
    assert_eq!(helper_calls, 2, "{:?}", parsed.calls);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_wgsl("fn ( { this is not valid wgsl @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.wgsl");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_wgsl(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Widget"),
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
    assert!(
        parsed.imports.iter().any(|i| i.module_path == "f16"),
        "{:?}",
        parsed.imports
    );
    Ok(())
}
