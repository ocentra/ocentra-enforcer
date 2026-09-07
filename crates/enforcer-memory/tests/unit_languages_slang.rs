//! Hard tests for Slang, onboarded directly through the generic
//! spec-table engine ([`enforcer_syntax::languages::generic::parse_slang`])
//! -- there is no bespoke `languages::slang` extractor to prove
//! zero-regression against (Slang has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::slang`]'s own doc comment
//! directly: this grammar's C++-shaped fields, fully reusing
//! [`enforcer_syntax::languages::generic::cpp_quirks`] verbatim (no
//! Slang-specific quirk exists at all).

use enforcer_syntax::languages::generic::parse_slang;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_slang";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_function_definition() {
    let src = r#"
float helper(float x) {
    return x;
}
"#;
    let parsed = parse_slang(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_struct_specifier() {
    let src = r#"
struct Widget {
    float label;
};
"#;
    let parsed = parse_slang(src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Widget"),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_import_statement_as_imports_edge() {
    let src = r#"
import WidgetLib;
"#;
    let parsed = parse_slang(src);
    assert!(!parsed.imports.is_empty(), "{:?}", parsed.imports);
}

#[test]
fn extracts_function_call_with_real_fields() -> TestResult {
    let src = r#"
float draw(float x) {
    return helper(x);
}
"#;
    let parsed = parse_slang(src);
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
float draw(float x) {
    if (x > 0.0) {
        return helper(x);
    }
    return helper(x);
}
"#;
    let parsed = parse_slang(src);
    let helper_calls = parsed.calls.iter().filter(|c| c.callee == "helper").count();
    assert_eq!(helper_calls, 2, "{:?}", parsed.calls);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_slang("float ( { this is not valid slang @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.slang");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_slang(&src);
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
    Ok(())
}
