//! Hard tests for Vue single-file components, onboarded directly
//! through the generic spec-table engine
//! ([`enforcer_memory::languages::generic::parse_vue`]). Grammar:
//! `tree-sitter-vue-next` 0.1.0, a real crates.io crate. Matches the
//! baseline's own `CBM_LANG_VUE` row -- `empty_types` for every array
//! except `module_types = {"document"}`; the embedded `<script>`
//! JS-import re-parse the baseline also wires is DEFERRED, see
//! [`enforcer_memory::languages::spec::LangSpec::vue`]'s own doc
//! comment for why.

use enforcer_memory::languages::generic::parse_vue;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_vue";

#[test]
fn extracts_one_module_symbol_for_the_document_root() -> TestResult {
    let src = fs::read_to_string(format!("{FIXTURE_DIR}/Sample.vue"))?;
    let parsed = parse_vue(&src);
    assert_eq!(parsed.symbols.len(), 1, "{:?}", parsed.symbols);
    assert_eq!(parsed.symbols[0].kind, SymbolKind::Module);
    Ok(())
}

#[test]
fn extracts_no_calls_imports_or_defines() -> TestResult {
    let src = fs::read_to_string(format!("{FIXTURE_DIR}/Sample.vue"))?;
    let parsed = parse_vue(&src);
    assert!(parsed.calls.is_empty());
    assert!(parsed.imports.is_empty());
    assert!(parsed.defines.is_empty());
    Ok(())
}
