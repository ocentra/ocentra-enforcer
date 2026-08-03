//! Hard tests for CMake, onboarded directly through the generic
//! spec-table engine ([`enforcer_syntax::languages::generic::parse_cmake`])
//! -- there is no bespoke `languages::cmake` extractor to prove
//! zero-regression against (CMake has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::cmake`]'s own doc comment
//! directly: this grammar is entirely field-free, so `function_def`/
//! `macro_def` name resolution and body recursion, plus `normal_command`
//! callee reconstruction, are all kind-search-based.

use enforcer_syntax::languages::generic::parse_cmake;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_cmake";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_function_def_as_function() {
    let src = "function(greet name)\n  message(\"hi\")\nendfunction()\n";
    let parsed = parse_cmake(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "greet"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_macro_def_as_function() {
    let src = "macro(setup)\n  greet(\"world\")\nendmacro()\n";
    let parsed = parse_cmake(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "setup"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_normal_command_callee() -> TestResult {
    let src = "project(Widget)\n";
    let parsed = parse_cmake(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "project")
        .ok_or("expected a project call")?;
    assert_eq!(call.arg_texts, vec!["Widget".to_string()], "{call:?}");
    Ok(())
}

#[test]
fn call_inside_function_def_body_records_from_symbol_scope() -> TestResult {
    let src = "function(greet name)\n  message(\"hi\")\nendfunction()\n";
    let parsed = parse_cmake(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "message")
        .ok_or("expected a nested message call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("greet"), "{call:?}");
    Ok(())
}

#[test]
fn no_import_types_recorded() {
    // `LangSpec::cmake`'s own `import_types` is deliberately empty (the
    // baseline itself has no dedicated CMake import-node array either --
    // `include`/`add_subdirectory` are ordinary `normal_command` calls,
    // not a distinct import statement kind).
    let src = "include(cmake/helpers.cmake)\n";
    let parsed = parse_cmake(src);
    assert!(parsed.imports.is_empty(), "{:?}", parsed.imports);
    assert!(
        parsed.calls.iter().any(|c| c.callee == "include"),
        "{:?}",
        parsed.calls
    );
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_cmake("function( ??? this is not valid cmake @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("CMakeLists.cmake");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_cmake(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "greet"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "setup"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.calls.iter().any(|c| c.callee == "project"),
        "{:?}",
        parsed.calls
    );
    assert!(
        parsed.calls.iter().any(|c| c.callee == "include"),
        "{:?}",
        parsed.calls
    );
    Ok(())
}
