//! Hard tests for Prisma, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_prisma`])
//! -- there is no bespoke `languages::prisma` extractor to prove
//! zero-regression against (Prisma has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::prisma`]'s own doc
//! comment directly: `model_declaration`/`enum_declaration`/... all
//! resolve their own name through a bare positional leading `identifier`
//! child, `column_declaration` DEFINES-edges into its enclosing model, and
//! `enum_declaration` is classified Enum (not folded into Class the way
//! baseline's own flat array does).

use enforcer_memory::languages::generic::parse_prisma;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_prisma";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_model_declaration_as_class_via_positional_name() {
    let src = "model User {\n  id Int @id\n}\n";
    let parsed = parse_prisma(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "User"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_column_declaration_as_defines_edge() {
    let src = "model User {\n  id Int @id\n  name String\n}\n";
    let parsed = parse_prisma(src);
    assert!(
        parsed
            .defines
            .iter()
            .any(|d| d.container_name == "User" && d.member_name == "name"),
        "{:?}",
        parsed.defines
    );
}

#[test]
fn extracts_enum_declaration_as_enum_kind_not_class() {
    let src = "enum Role {\n  USER\n  ADMIN\n}\n";
    let parsed = parse_prisma(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Role"),
        Some(&SymbolKind::Enum),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_call_expression_inside_field_attribute() -> TestResult {
    let src = "model User {\n  id Int @id @default(autoincrement())\n}\n";
    let parsed = parse_prisma(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "autoincrement")
        .ok_or("expected an autoincrement() call")?;
    let _ = call;
    Ok(())
}

#[test]
fn extracts_datasource_declaration_as_class() {
    let src = "datasource db {\n  provider = \"postgresql\"\n}\n";
    let parsed = parse_prisma(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "db"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn parses_fixture_sample_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("sample.prisma");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_prisma(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "User"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Role"),
        Some(&SymbolKind::Enum),
        "{:?}",
        parsed.symbols
    );
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "autoincrement" || c.callee == "env")
        .ok_or("expected at least one call_expression")?;
    let _ = call;
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "model User {\n  id Int @id\n}\n";
    let first = parse_prisma(src);
    let second = parse_prisma(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_prisma("model ( { this is not valid prisma @@@");
    let _ = parsed;
}
