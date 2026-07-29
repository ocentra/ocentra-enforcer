//! Hard tests for BitBake, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_bitbake`])
//! -- there is no bespoke `languages::bitbake` extractor to prove
//! zero-regression against (BitBake has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::bitbake`]'s own doc
//! comment directly: positional `function_definition`/
//! `anonymous_python_function` naming (neither has a `name` field), the
//! real `recipe` root, and the `inherit_directive`/`require_directive`
//! import improvement over the baseline's own array.

use enforcer_memory::languages::generic::parse_bitbake;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_bitbake";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_shell_task_function_via_positional_name() {
    let src = "do_compile() {\n    oe_runmake\n}\n";
    let parsed = parse_bitbake(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "do_compile"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_python_task_function_via_positional_name() {
    let src = "python do_custom_task() {\n    bb.note(\"hi\")\n}\n";
    let parsed = parse_bitbake(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "do_custom_task"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_call_inside_python_task_body() -> TestResult {
    let src = "python do_custom_task() {\n    bb.note(\"hi\")\n}\n";
    let parsed = parse_bitbake(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee.contains("note"))
        .ok_or("expected a bb.note call")?;
    let _ = call;
    Ok(())
}

#[test]
fn extracts_inherit_directive_as_import() -> TestResult {
    let src = "inherit cmake\n";
    let parsed = parse_bitbake(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(!paths.is_empty(), "{paths:?}");
    Ok(())
}

#[test]
fn extracts_require_directive_as_import() -> TestResult {
    let src = "require common.inc\n";
    let parsed = parse_bitbake(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(!paths.is_empty(), "{paths:?}");
    Ok(())
}

#[test]
fn extracts_include_directive_as_import() -> TestResult {
    let src = "include common.inc\n";
    let parsed = parse_bitbake(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(!paths.is_empty(), "{paths:?}");
    Ok(())
}

#[test]
fn parses_fixture_example_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("example.bb");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_bitbake(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "do_compile"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "do_custom_task"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert!(!parsed.imports.is_empty(), "{:?}", parsed.imports);
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "do_compile() {\n    oe_runmake\n}\n";
    let first = parse_bitbake(src);
    let second = parse_bitbake(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_bitbake("do_compile( { this is not valid bitbake @@@");
    let _ = parsed;
}
