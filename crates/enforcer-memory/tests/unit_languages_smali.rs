//! Hard tests for Smali (Android bytecode disassembly text format),
//! onboarded directly through the generic spec-table engine
//! ([`enforcer_syntax::languages::generic::parse_smali`]) -- grammar
//! VENDORED (`vendor/tree-sitter-smali-local/`). Asserts against the
//! grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::smali`]'s own doc
//! comment: `class_definition` is claimed WHOLESALE by
//! [`enforcer_syntax::languages::generic::smali_quirk`] (every node kind
//! in this grammar is completely fieldless), which resolves the class/
//! method/field/import symbols and DEFINES edges in one pass.

use enforcer_syntax::languages::generic::parse_smali;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_smali";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

const FOO_SRC: &str = ".class public LFoo;\n.super Ljava/lang/Object;\n\n.method public add(II)I\n    .locals 1\n    add-int v0, p1, p2\n    return v0\n.end method\n";

#[test]
fn extracts_class_name_via_wholesale_quirk() {
    let parsed = parse_smali(FOO_SRC);
    assert_eq!(
        symbol_kind(&parsed.symbols, "LFoo;"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_method_name_via_two_level_descent() {
    let parsed = parse_smali(FOO_SRC);
    assert_eq!(
        symbol_kind(&parsed.symbols, "add"),
        Some(&SymbolKind::Method),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_method_defines_edge() -> TestResult {
    let parsed = parse_smali(FOO_SRC);
    parsed
        .defines
        .iter()
        .find(|d| d.container_name == "LFoo;" && d.member_name == "add")
        .ok_or("expected a LFoo;->add DEFINES edge")?;
    Ok(())
}

#[test]
fn extracts_super_directive_as_import() -> TestResult {
    let parsed = parse_smali(FOO_SRC);
    parsed
        .imports
        .iter()
        .find(|i| i.module_path.contains("Object"))
        .ok_or("expected a super-class import")?;
    Ok(())
}

#[test]
fn parses_fixture_foo_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("Foo.smali");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_smali(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "LFoo;"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "add"),
        Some(&SymbolKind::Method),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "main"),
        Some(&SymbolKind::Method),
        "{:?}",
        parsed.symbols
    );
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let first = parse_smali(FOO_SRC);
    let second = parse_smali(FOO_SRC);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_smali("this is not valid smali @@@ ### <<<");
    let _ = parsed;
}
