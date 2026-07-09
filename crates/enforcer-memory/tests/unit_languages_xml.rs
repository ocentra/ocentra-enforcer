//! Hard tests for XML, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_xml`]).
//! Grammar: `tree-sitter-xml` 0.7.0's own `LANGUAGE_XML` entry point, a
//! real crates.io crate. Matches the baseline's own `CBM_LANG_XML` row's
//! `module_types = {"document"}`/`class_types = {"element"}` -- `element`
//! is entirely fieldless in this real grammar, so
//! [`enforcer_memory::languages::generic::xml_quirk`] finds the tag name
//! two levels down (`element` -> `STag`/`EmptyElemTag` -> `Name`) rather
//! than the generic engine's own field-based fallback, see
//! [`enforcer_memory::languages::spec::LangSpec::xml`]'s own doc
//! comment.

use enforcer_memory::languages::generic::parse_xml;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_xml";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_the_root_element_as_a_class_symbol() -> TestResult {
    let src = fs::read_to_string(format!("{FIXTURE_DIR}/sample.xml"))?;
    let parsed = parse_xml(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "note"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    Ok(())
}

#[test]
fn extracts_nested_child_elements_as_class_symbols_with_defines_edges() -> TestResult {
    let src = fs::read_to_string(format!("{FIXTURE_DIR}/sample.xml"))?;
    let parsed = parse_xml(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "to"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    parsed
        .defines
        .iter()
        .find(|d| d.container_name == "note" && d.member_name == "to")
        .ok_or("expected a note->to DEFINES edge")?;
    parsed
        .defines
        .iter()
        .find(|d| d.container_name == "note" && d.member_name == "from")
        .ok_or("expected a note->from DEFINES edge")?;
    Ok(())
}

#[test]
fn extracts_no_calls_or_imports() -> TestResult {
    let src = fs::read_to_string(format!("{FIXTURE_DIR}/sample.xml"))?;
    let parsed = parse_xml(&src);
    assert!(parsed.calls.is_empty());
    assert!(parsed.imports.is_empty());
    Ok(())
}
