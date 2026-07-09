//! Hard tests for SOQL (Salesforce Object Query Language), onboarded
//! directly through the generic spec-table engine
//! ([`enforcer_memory::languages::generic::parse_soql`]). Grammar:
//! `tree-sitter-sfapex` 3.0.0's own `soql` module, the same crate
//! already a dependency for Apex. Matches the baseline's own
//! `CBM_LANG_SOQL` row -- `empty_types` for every array except
//! `module_types = {"source_file"}`.

use enforcer_memory::languages::generic::parse_soql;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_soql";

#[test]
fn extracts_one_module_symbol_for_the_source_file_root() -> TestResult {
    let src = fs::read_to_string(format!("{FIXTURE_DIR}/sample.soql"))?;
    let parsed = parse_soql(&src);
    assert_eq!(parsed.symbols.len(), 1, "{:?}", parsed.symbols);
    assert_eq!(parsed.symbols[0].kind, SymbolKind::Module);
    Ok(())
}

#[test]
fn extracts_no_calls_imports_or_defines() -> TestResult {
    let src = fs::read_to_string(format!("{FIXTURE_DIR}/sample.soql"))?;
    let parsed = parse_soql(&src);
    assert!(parsed.calls.is_empty());
    assert!(parsed.imports.is_empty());
    assert!(parsed.defines.is_empty());
    Ok(())
}
