//! Hard tests for INI, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_memory::languages::generic::parse_ini`]) -- grammar:
//! `tree-sitter-ini` 1.4.0. Asserts against the grammar-shape ground
//! truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::ini`]'s own doc
//! comment: `section`'s fieldless `section_name` claimed by
//! [`enforcer_memory::languages::generic::ini_quirk`], plus each
//! `setting` promoted to a DEFINES edge under its own section.

use enforcer_memory::languages::generic::parse_ini;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_ini";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_section_via_quirk() {
    let src = "[section]\nkey = value\n";
    let parsed = parse_ini(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "section"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_setting_as_defines_member() -> TestResult {
    let src = "[section]\nkey = value\n";
    let parsed = parse_ini(src);
    parsed
        .defines
        .iter()
        .find(|d| d.container_name == "section" && d.member_name == "key")
        .ok_or("expected a DEFINES edge for section.key")?;
    Ok(())
}

#[test]
fn parses_fixture_settings_ini_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("settings.ini");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_ini(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "section"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "section2"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "[a]\nx = 1\n";
    let first = parse_ini(src);
    let second = parse_ini(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_ini("[[[ not valid @@@");
    let _ = parsed;
}
