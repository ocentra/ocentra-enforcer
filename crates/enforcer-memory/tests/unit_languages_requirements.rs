//! Hard tests for pip `requirements.txt` files, onboarded directly
//! through the generic spec-table engine
//! ([`enforcer_memory::languages::generic::parse_requirements`]).
//! Grammar: `tree-sitter-requirements` 0.6.1, a real crates.io crate.
//! Matches the baseline's own `CBM_LANG_REQUIREMENTS` row -- `empty_types`
//! for every array except `module_types = {"file"}` -- see
//! [`enforcer_memory::languages::spec::LangSpec::requirements`]'s own doc
//! comment.

use enforcer_memory::languages::generic::parse_requirements;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_requirements";

#[test]
fn extracts_one_module_symbol_for_the_file_root() -> TestResult {
    let src = fs::read_to_string(format!("{FIXTURE_DIR}/requirements.txt"))?;
    let parsed = parse_requirements(&src);
    assert_eq!(parsed.symbols.len(), 1, "{:?}", parsed.symbols);
    assert_eq!(parsed.symbols[0].kind, SymbolKind::Module);
    Ok(())
}

#[test]
fn extracts_no_calls_imports_or_defines() -> TestResult {
    let src = fs::read_to_string(format!("{FIXTURE_DIR}/requirements.txt"))?;
    let parsed = parse_requirements(&src);
    assert!(parsed.calls.is_empty());
    assert!(parsed.imports.is_empty());
    assert!(parsed.defines.is_empty());
    Ok(())
}
