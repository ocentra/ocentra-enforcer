//! Hard tests for YAML, onboarded directly through the generic
//! spec-table engine ([`enforcer_syntax::languages::generic::parse_yaml`]).
//! Grammar: `tree-sitter-yaml` 0.7.2, a real crates.io crate. Matches
//! the baseline's own `CBM_LANG_YAML` row -- `empty_types` for every
//! array except `module_types = {"stream"}` (the real file root, one
//! level ABOVE the grammar's own `document` node -- see
//! [`enforcer_syntax::languages::spec::LangSpec::yaml`]'s own doc
//! comment).

use enforcer_syntax::languages::generic::parse_yaml;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_yaml";

#[test]
fn extracts_one_module_symbol_for_the_stream_root() -> TestResult {
    let src = fs::read_to_string(format!("{FIXTURE_DIR}/sample.yaml"))?;
    let parsed = parse_yaml(&src);
    assert_eq!(parsed.symbols.len(), 1, "{:?}", parsed.symbols);
    assert_eq!(parsed.symbols[0].kind, SymbolKind::Module);
    Ok(())
}

#[test]
fn extracts_no_calls_imports_or_defines() -> TestResult {
    let src = fs::read_to_string(format!("{FIXTURE_DIR}/sample.yaml"))?;
    let parsed = parse_yaml(&src);
    assert!(parsed.calls.is_empty());
    assert!(parsed.imports.is_empty());
    assert!(parsed.defines.is_empty());
    Ok(())
}
