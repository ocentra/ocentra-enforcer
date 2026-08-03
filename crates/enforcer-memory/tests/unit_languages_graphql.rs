//! Hard tests for GraphQL SDL, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_syntax::languages::generic::parse_graphql`]) -- grammar:
//! `tree-sitter-graphql` 0.1.0. Asserts against the grammar-shape
//! ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::graphql`]'s own doc
//! comment: every type-definition kind's fieldless `name` child,
//! resolved by [`enforcer_syntax::languages::generic::graphql_quirk`],
//! plus its manual `fields_definition`/`input_fields_definition` walk
//! for member DEFINES edges.

use enforcer_syntax::languages::generic::parse_graphql;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_graphql";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_object_type_definition_via_quirk() {
    let src = "type User {\n  id: ID!\n  name: String\n}\n";
    let parsed = parse_graphql(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "User"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_field_definition_as_defines_member() -> TestResult {
    let src = "type User {\n  id: ID!\n  name: String\n}\n";
    let parsed = parse_graphql(src);
    parsed
        .defines
        .iter()
        .find(|d| d.container_name == "User" && d.member_name == "name")
        .ok_or("expected a DEFINES edge for User.name")?;
    Ok(())
}

#[test]
fn extracts_input_object_type_definition_and_its_fields() -> TestResult {
    let src = "input Filter {\n  term: String\n}\n";
    let parsed = parse_graphql(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Filter"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    parsed
        .defines
        .iter()
        .find(|d| d.container_name == "Filter" && d.member_name == "term")
        .ok_or("expected a DEFINES edge for Filter.term")?;
    Ok(())
}

#[test]
fn parses_fixture_schema_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("schema.graphql");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_graphql(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Query"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "User"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "type Query {\n  ping: String\n}\n";
    let first = parse_graphql(src);
    let second = parse_graphql(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_graphql("type [[[ not valid @@@");
    let _ = parsed;
}
