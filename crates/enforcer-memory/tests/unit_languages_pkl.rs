//! Hard tests for Pkl, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_pkl`])
//! -- there is no bespoke `languages::pkl` extractor to prove
//! zero-regression against (Pkl has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::pkl`]'s own doc comment
//! directly: `clazz`'s own name is a bare positional `identifier` child,
//! `classMethod`'s own name lives two levels down on its `methodHeader`
//! child, and `importClause`/`extendsOrAmendsClause` both resolve their
//! own path through a `stringConstant` child (this grammar's own bare
//! `import` keyword token carries no path text of its own).

use enforcer_memory::languages::generic::parse_pkl;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_pkl";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_clazz_as_class_via_positional_name() {
    let src = "class Person {\n  name: String\n}\n";
    let parsed = parse_pkl(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Person"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_class_method_as_method_via_method_header_descent() {
    let src = "class Person {\n  function greet(): String = \"hi\"\n}\n";
    let parsed = parse_pkl(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "greet"),
        Some(&SymbolKind::Method),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed
            .defines
            .iter()
            .any(|d| d.container_name == "Person" && d.member_name == "greet"),
        "{:?}",
        parsed.defines
    );
}

#[test]
fn top_level_function_is_classified_function_not_method() {
    let src = "function double(x: Int): Int = x * 2\n";
    let parsed = parse_pkl(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "double"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_import_clause_path_via_string_constant() {
    let src = "import \"pkl:test\"\n\nx = 1\n";
    let parsed = parse_pkl(src);
    assert!(
        parsed.imports.iter().any(|i| i.module_path == "pkl:test"),
        "{:?}",
        parsed.imports
    );
}

#[test]
fn extracts_extends_or_amends_clause_as_import() {
    let src = "extends \"base.pkl\"\n\nclass Animal {\n}\n";
    let parsed = parse_pkl(src);
    assert!(
        parsed.imports.iter().any(|i| i.module_path == "base.pkl"),
        "{:?}",
        parsed.imports
    );
}

#[test]
fn parses_fixture_sample_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("sample.pkl");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_pkl(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Person"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "greet"),
        Some(&SymbolKind::Method),
        "{:?}",
        parsed.symbols
    );
    let _ = parsed
        .imports
        .iter()
        .find(|i| i.module_path == "pkl:test")
        .ok_or("expected pkl:test import")?;
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "class Person {\n  name: String\n}\n";
    let first = parse_pkl(src);
    let second = parse_pkl(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_pkl("class ( { this is not valid pkl @@@");
    let _ = parsed;
}
