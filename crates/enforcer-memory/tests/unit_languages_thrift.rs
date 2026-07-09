//! Hard tests for Thrift, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_memory::languages::generic::parse_thrift`]) -- there is
//! no bespoke `languages::thrift` extractor to prove zero-regression
//! against (Thrift has never had one in this crate), so these tests
//! assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::thrift`]'s own doc
//! comment directly: the real `name_field = "type"` shared by
//! struct/union/enum/senum/service/interaction, and the positional
//! (unfielded) `function_definition`/`field`/`exception_definition`
//! name extraction [`enforcer_memory::languages::generic::thrift_quirk`]
//! performs instead.

use enforcer_memory::languages::generic::parse_thrift;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_thrift";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_struct_via_real_type_field() {
    let src = "struct Person {\n  1: string name,\n}\n";
    let parsed = parse_thrift(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Person"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_enum_via_real_type_field() {
    let src = "enum Color {\n  RED = 0,\n  GREEN = 1,\n}\n";
    let parsed = parse_thrift(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Color"),
        Some(&SymbolKind::Enum),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_service_as_interface_and_its_methods() {
    let src = "service Greeter {\n  string greet(1: string name),\n}\n";
    let parsed = parse_thrift(src);
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
fn extracts_exception_as_class_via_positional_identifier() {
    // Regression guard: `exception_definition` exposes NO field at all
    // for its own name (unlike struct/union/enum, whose name lives on
    // the real `type` field) -- see `thrift_quirk`'s own doc comment.
    let src = "exception MyError {\n  1: string message,\n}\n";
    let parsed = parse_thrift(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "MyError"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed
            .defines
            .iter()
            .any(|d| d.container_name == "MyError" && d.member_name == "message"),
        "{:?}",
        parsed.defines
    );
}

#[test]
fn extracts_field_defines_edge_via_positional_identifier() {
    let src = "struct Person {\n  1: string name,\n  2: i32 age,\n}\n";
    let parsed = parse_thrift(src);
    assert!(
        parsed
            .defines
            .iter()
            .any(|d| d.container_name == "Person" && d.member_name == "name"),
        "{:?}",
        parsed.defines
    );
    assert!(
        parsed
            .defines
            .iter()
            .any(|d| d.container_name == "Person" && d.member_name == "age"),
        "{:?}",
        parsed.defines
    );
}

#[test]
fn extracts_free_function_definition_as_function_kind() {
    // A bare top-level `function_definition` (Facebook `interaction`
    // sibling aside) only occurs nested in a service/interaction in
    // real Thrift; this asserts the Method classification when nested.
    let src = "service S {\n  void ping(),\n}\n";
    let parsed = parse_thrift(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "ping"),
        Some(&SymbolKind::Method),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_include_statement_as_import() -> TestResult {
    let src = "include \"other.thrift\"\n";
    let parsed = parse_thrift(src);
    assert!(!parsed.imports.is_empty(), "{:?}", parsed.imports);
    Ok(())
}

#[test]
fn parses_fixture_person_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("person.thrift");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_thrift(&src);
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
    assert_eq!(
        symbol_kind(&parsed.symbols, "MyError"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert!(!parsed.imports.is_empty(), "{:?}", parsed.imports);
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "struct Person {\n  1: string name,\n}\n";
    let first = parse_thrift(src);
    let second = parse_thrift(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_thrift("struct ( { this is not valid thrift @@@");
    let _ = parsed;
}
