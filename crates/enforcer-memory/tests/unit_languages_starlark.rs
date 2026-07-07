//! Hard tests for Starlark, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_starlark`])
//! -- there is no bespoke `languages::starlark` extractor to prove
//! zero-regression against (Starlark has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::starlark`]'s own doc
//! comment directly: `function_definition`'s ordinary `name`/`body`
//! fields, `call`'s ordinary `function`/`arguments` fields, and
//! `load(...)`'s `call_override`-caught IMPORTS detection (the baseline's
//! own `starlark_import_types` array entry, `"with_clause"`, is a real but
//! unrelated node -- see that doc comment for the full finding).

use enforcer_memory::languages::generic::parse_starlark;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_starlark";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_function_definition() {
    let src = r#"
def helper(x):
    return x + 1
"#;
    let parsed = parse_starlark(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_load_statement_as_imports_edge() {
    let src = r#"
load("//tools/build_defs:widget.bzl", "widget_library")
"#;
    let parsed = parse_starlark(src);
    assert!(
        parsed
            .imports
            .iter()
            .any(|i| i.module_path == "//tools/build_defs:widget.bzl"),
        "{:?}",
        parsed.imports
    );
}

#[test]
fn load_call_also_records_an_ordinary_calls_edge() {
    let src = r#"
load("//tools/build_defs:widget.bzl", "widget_library")
"#;
    let parsed = parse_starlark(src);
    assert!(
        parsed.calls.iter().any(|c| c.callee == "load"),
        "{:?}",
        parsed.calls
    );
}

#[test]
fn with_statement_is_not_treated_as_an_import() {
    // Regression guard for the baseline array bug this row corrects: a
    // plain Python-style `with` context manager must NOT produce an
    // IMPORTS edge (its `with_clause` child is unrelated to Starlark's
    // real `load(...)` import mechanism).
    let src = r#"
def helper(f):
    with f as g:
        return g
"#;
    let parsed = parse_starlark(src);
    assert!(parsed.imports.is_empty(), "{:?}", parsed.imports);
}

#[test]
fn extracts_function_call_with_real_fields() -> TestResult {
    let src = r#"
def draw(x):
    helper(x)
"#;
    let parsed = parse_starlark(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.arg_texts, vec!["x".to_string()], "{call:?}");
    assert_eq!(call.from_symbol.as_deref(), Some("draw"), "{call:?}");
    Ok(())
}

#[test]
fn if_statement_is_recognized_as_a_branch_node() {
    let src = r#"
def draw(x):
    if x:
        helper(x)
    else:
        helper(x)
"#;
    let parsed = parse_starlark(src);
    let helper_calls = parsed.calls.iter().filter(|c| c.callee == "helper").count();
    assert_eq!(helper_calls, 2, "{:?}", parsed.calls);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_starlark("def ( { this is not valid starlark @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.bzl");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_starlark(&src);
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
        parsed
            .imports
            .iter()
            .any(|i| i.module_path.contains("widget.bzl")),
        "{:?}",
        parsed.imports
    );
    Ok(())
}
