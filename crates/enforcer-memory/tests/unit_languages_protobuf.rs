//! Hard tests for Protobuf, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_protobuf`])
//! -- there is no bespoke `languages::protobuf` extractor to prove
//! zero-regression against (Protobuf has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::protobuf`]'s own doc
//! comment directly: `message`/`service`/`enum`/`rpc` all resolve their
//! own name through a dedicated `*_name` wrapper child, `field`/
//! `map_field` DEFINES-edge into their enclosing `message`, and `import`'s
//! `path` field is read via the quirk (the generic engine has no
//! field-based import fallback of its own).

use enforcer_memory::languages::generic::parse_protobuf;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_protobuf";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_message_as_class_via_message_name_wrapper() {
    let src = "message Foo {\n  string name = 1;\n}\n";
    let parsed = parse_protobuf(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Foo"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_field_as_defines_edge_into_enclosing_message() {
    let src = "message Foo {\n  string name = 1;\n}\n";
    let parsed = parse_protobuf(src);
    assert!(
        parsed
            .defines
            .iter()
            .any(|d| d.container_name == "Foo" && d.member_name == "name"),
        "{:?}",
        parsed.defines
    );
}

#[test]
fn extracts_service_and_rpc_with_defines_edge() {
    let src = "service Greeter {\n  rpc SayHello (Foo) returns (Foo);\n}\n";
    let parsed = parse_protobuf(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Greeter"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "SayHello"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed
            .defines
            .iter()
            .any(|d| d.container_name == "Greeter" && d.member_name == "SayHello"),
        "{:?}",
        parsed.defines
    );
}

#[test]
fn extracts_enum_as_enum_kind() {
    let src = "enum Status {\n  OK = 0;\n}\n";
    let parsed = parse_protobuf(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Status"),
        Some(&SymbolKind::Enum),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_import_path_without_quotes() {
    let src = "import \"other.proto\";\n";
    let parsed = parse_protobuf(src);
    assert!(
        parsed
            .imports
            .iter()
            .any(|i| i.module_path == "other.proto"),
        "{:?}",
        parsed.imports
    );
}

#[test]
fn parses_fixture_sample_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("sample.proto");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_protobuf(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Foo"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Greeter"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Status"),
        Some(&SymbolKind::Enum),
        "{:?}",
        parsed.symbols
    );
    let _ = parsed
        .imports
        .iter()
        .find(|i| i.module_path == "other.proto")
        .ok_or("expected other.proto import")?;
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "message Foo {\n  string name = 1;\n}\n";
    let first = parse_protobuf(src);
    let second = parse_protobuf(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_protobuf("message ( { this is not valid proto @@@");
    let _ = parsed;
}
