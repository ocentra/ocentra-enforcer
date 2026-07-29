//! Hard tests for reStructuredText, onboarded directly through the
//! generic spec-table engine
//! ([`enforcer_memory::languages::generic::parse_rst`]). Grammar:
//! `tree-sitter-rst` 0.2.0, a real crates.io crate (directly compatible
//! with this workspace's `tree-sitter` core, no vendoring needed).
//! Matches the baseline's own `CBM_LANG_RST` row -- `empty_types` for
//! every array except `module_types = {"document"}`.

use enforcer_memory::languages::generic::parse_rst;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_rst";

#[test]
fn extracts_one_module_symbol_for_the_document_root() -> TestResult {
    let src = fs::read_to_string(format!("{FIXTURE_DIR}/sample.rst"))?;
    let parsed = parse_rst(&src);
    assert_eq!(parsed.symbols.len(), 1, "{:?}", parsed.symbols);
    assert_eq!(parsed.symbols[0].kind, SymbolKind::Module);
    Ok(())
}

#[test]
fn extracts_no_calls_imports_or_defines() -> TestResult {
    let src = fs::read_to_string(format!("{FIXTURE_DIR}/sample.rst"))?;
    let parsed = parse_rst(&src);
    assert!(parsed.calls.is_empty());
    assert!(parsed.imports.is_empty());
    assert!(parsed.defines.is_empty());
    Ok(())
}
