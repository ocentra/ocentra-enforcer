//! Hard tests for Cap'n Proto, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_memory::languages::generic::parse_capnp`]) -- there is
//! no bespoke `languages::capnp` extractor to prove zero-regression
//! against (Cap'n Proto has never had one in this crate), so these
//! tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::capnp`]'s own doc
//! comment directly: every claimed node kind exposes ZERO fields at
//! all (confirmed via a real `node-types.json` dump), so names are
//! read positionally by [`enforcer_memory::languages::generic::capnp_quirk`],
//! and nested in-place type definitions parse as
//! `field > nested_struct > struct`.

use enforcer_memory::languages::generic::parse_capnp;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_capnp";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_struct_via_positional_type_identifier() {
    let src = "struct Person {\n  name @0 :Text;\n}\n";
    let parsed = parse_capnp(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Person"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_interface_and_method_via_positional_identifiers() {
    let src = "interface Greeter {\n  greet @0 (name :Text) -> (reply :Text);\n}\n";
    let parsed = parse_capnp(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Greeter"),
        Some(&SymbolKind::Interface),
        "{:?}",
        parsed.symbols
    );
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
            .any(|d| d.container_name == "Greeter" && d.member_name == "greet"),
        "{:?}",
        parsed.defines
    );
}

#[test]
fn extracts_enum_via_positional_enum_identifier() {
    let src = "enum Color {\n  red @0;\n  green @1;\n}\n";
    let parsed = parse_capnp(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Color"),
        Some(&SymbolKind::Enum),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_nested_struct_reached_through_wrapping_field() {
    // Regression guard for the confirmed real-grammar finding (see
    // `LangSpec::capnp`'s own doc comment): a nested struct definition
    // parses as `field > nested_struct > struct`, not as a direct
    // child of the outer struct -- only reachable if `field`
    // unconditionally recurses.
    let src = "struct Outer {\n  struct Inner {\n    x @0 :Int32;\n  }\n}\n";
    let parsed = parse_capnp(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Outer"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Inner"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_field_defines_edge() {
    let src = "struct Person {\n  name @0 :Text;\n}\n";
    let parsed = parse_capnp(src);
    assert!(
        parsed
            .defines
            .iter()
            .any(|d| d.container_name == "Person" && d.member_name == "name"),
        "{:?}",
        parsed.defines
    );
}

#[test]
fn extracts_const_as_constant_symbol() {
    let src = "const globalConst :Int32 = 42;\n";
    let parsed = parse_capnp(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "globalConst"),
        Some(&SymbolKind::Constant),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_using_directive_as_import() -> TestResult {
    let src = "using Cxx = import \"/capnp/c++.capnp\";\n";
    let parsed = parse_capnp(src);
    assert!(!parsed.imports.is_empty(), "{:?}", parsed.imports);
    Ok(())
}

#[test]
fn parses_fixture_person_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("person.capnp");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_capnp(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Person"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Greeter"),
        Some(&SymbolKind::Interface),
        "{:?}",
        parsed.symbols
    );
    assert!(!parsed.imports.is_empty(), "{:?}", parsed.imports);
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "struct Person {\n  name @0 :Text;\n}\n";
    let first = parse_capnp(src);
    let second = parse_capnp(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_capnp("struct ( { this is not valid capnp @@@");
    let _ = parsed;
}
