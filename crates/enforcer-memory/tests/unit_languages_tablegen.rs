//! Hard tests for LLVM TableGen, onboarded directly through the
//! generic spec-table engine
//! ([`enforcer_syntax::languages::generic::parse_tablegen`]) -- there
//! is no bespoke `languages::tablegen` extractor to prove
//! zero-regression against (TableGen has never had one in this
//! crate), so these tests assert against the grammar-shape ground
//! truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::tablegen`]'s own doc
//! comment directly: `def`/`class`/`multiclass`/`defm` all genuinely
//! carry a real `"name"` field (the one fully-correct baseline array
//! set of this whole wave), and the positional (unfielded)
//! `include_directive` path
//! [`enforcer_syntax::languages::generic::tablegen_quirk`] extracts.

use enforcer_syntax::languages::generic::parse_tablegen;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_tablegen";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_class_via_real_name_field() {
    let src = "class Instruction {\n  string Name;\n}\n";
    let parsed = parse_tablegen(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Instruction"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_def_via_real_name_field() {
    let src = "class Instruction {\n  string Name;\n}\n\ndef MyInstr : Instruction {\n  let Name = \"myinstr\";\n}\n";
    let parsed = parse_tablegen(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "MyInstr"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_multiclass_and_defm_via_real_name_field() {
    let src = "class Instruction {}\n\nmulticlass Foo<string name> {\n  def _rr : Instruction;\n}\n\ndefm Bar : Foo<\"bar\">;\n";
    let parsed = parse_tablegen(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Foo"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Bar"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_include_directive_as_import_via_positional_string() -> TestResult {
    let src = "include \"llvm/Target/Target.td\"\n";
    let parsed = parse_tablegen(src);
    assert!(!parsed.imports.is_empty(), "{:?}", parsed.imports);
    Ok(())
}

#[test]
fn parses_fixture_example_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("example.td");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_tablegen(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Instruction"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "MyInstr"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Foo"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Bar"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert!(!parsed.imports.is_empty(), "{:?}", parsed.imports);
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "class Instruction {}\n\ndef MyInstr : Instruction;\n";
    let first = parse_tablegen(src);
    let second = parse_tablegen(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_tablegen("class ( { this is not valid tablegen @@@");
    let _ = parsed;
}
