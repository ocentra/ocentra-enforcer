//! Hard tests for TOML, onboarded directly through the generic
//! spec-table engine ([`enforcer_syntax::languages::generic::parse_toml`]).
//! Grammar: `tree-sitter-toml-ng` 0.7.0, a real crates.io crate. Matches
//! the baseline's own `CBM_LANG_TOML` row's `module_types = {"document"}`/
//! `class_types = {"table", "table_array_element"}` -- both node kinds
//! are entirely fieldless in this real grammar, so
//! [`enforcer_syntax::languages::generic::toml_quirk`] claims them by
//! positional key lookup rather than the generic engine's own
//! field-based fallback, see
//! [`enforcer_syntax::languages::spec::LangSpec::toml`]'s own doc
//! comment.

use enforcer_syntax::languages::generic::parse_toml;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_toml";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_document_root_as_a_module_symbol() -> TestResult {
    let src = fs::read_to_string(format!("{FIXTURE_DIR}/sample.toml"))?;
    let parsed = parse_toml(&src);
    let modules: Vec<_> = parsed
        .symbols
        .iter()
        .filter(|s| s.kind == SymbolKind::Module)
        .collect();
    assert_eq!(modules.len(), 1, "{:?}", parsed.symbols);
    Ok(())
}

#[test]
fn extracts_table_header_as_a_class_symbol() -> TestResult {
    let src = fs::read_to_string(format!("{FIXTURE_DIR}/sample.toml"))?;
    let parsed = parse_toml(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "package"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    Ok(())
}

#[test]
fn extracts_table_array_element_header_as_a_class_symbol() -> TestResult {
    let src = fs::read_to_string(format!("{FIXTURE_DIR}/sample.toml"))?;
    let parsed = parse_toml(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "bin"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    Ok(())
}

#[test]
fn extracts_no_calls_or_imports() -> TestResult {
    let src = fs::read_to_string(format!("{FIXTURE_DIR}/sample.toml"))?;
    let parsed = parse_toml(&src);
    assert!(parsed.calls.is_empty());
    assert!(parsed.imports.is_empty());
    Ok(())
}
