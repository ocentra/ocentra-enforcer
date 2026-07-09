//! Hard tests for RON (Rusty Object Notation), onboarded directly
//! through the generic spec-table engine
//! ([`enforcer_memory::languages::generic::parse_ron`]). Grammar
//! VENDORED (`vendor/tree-sitter-ron-local/`) -- the published
//! `tree-sitter-ron` crate's own binding pins an incompatible
//! `tree-sitter` version, see
//! [`enforcer_memory::languages::spec::LangSpec::ron`]'s own doc
//! comment. Matches the baseline's own `CBM_LANG_RON` row --
//! `empty_types` for every array except `module_types = {"source_file"}`.

use enforcer_memory::languages::generic::parse_ron;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_ron";

#[test]
fn extracts_one_module_symbol_for_the_source_file_root() -> TestResult {
    let src = fs::read_to_string(format!("{FIXTURE_DIR}/sample.ron"))?;
    let parsed = parse_ron(&src);
    assert_eq!(parsed.symbols.len(), 1, "{:?}", parsed.symbols);
    assert_eq!(parsed.symbols[0].kind, SymbolKind::Module);
    Ok(())
}

#[test]
fn extracts_no_calls_imports_or_defines() -> TestResult {
    let src = fs::read_to_string(format!("{FIXTURE_DIR}/sample.ron"))?;
    let parsed = parse_ron(&src);
    assert!(parsed.calls.is_empty());
    assert!(parsed.imports.is_empty());
    assert!(parsed.defines.is_empty());
    Ok(())
}
