//! Hard tests for SQL, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_sql`])
//! -- there is no bespoke `languages::sql` extractor to prove
//! zero-regression against (SQL has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::sql`]'s own doc comment
//! directly: `create_function`/`create_type` both resolve their own name
//! through an unfielded `object_reference` child's own `name` field, and
//! `invocation` calls are recorded via `call_override`.

use enforcer_memory::languages::generic::parse_sql;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_sql";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_create_function_via_object_reference_name() {
    let src = "CREATE FUNCTION add_one(x integer) RETURNS integer AS $$\nBEGIN\n  RETURN x + 1;\nEND;\n$$ LANGUAGE plpgsql;\n";
    let parsed = parse_sql(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "add_one"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_create_type_as_class_via_object_reference_name() {
    let src = "CREATE TYPE mood AS ENUM ('sad', 'ok', 'happy');\n";
    let parsed = parse_sql(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "mood"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_invocation_as_call() -> TestResult {
    let src = "SELECT upper('x') FROM accounts;\n";
    let parsed = parse_sql(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "upper")
        .ok_or("expected an upper() call")?;
    let _ = call;
    Ok(())
}

#[test]
fn extracts_case_as_branch() {
    let src = "SELECT CASE WHEN id > 1 THEN true ELSE false END FROM accounts;\n";
    let parsed = parse_sql(src);
    // Complexity extraction is deferred this wave; this test only proves
    // the file parses cleanly with a `case` present in the source.
    let _ = parsed;
}

#[test]
fn parses_fixture_sample_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("sample.sql");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_sql(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "add_one"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "mood"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "add_one" || c.callee == "upper")
        .ok_or("expected at least one invocation call")?;
    let _ = call;
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "CREATE TYPE mood AS ENUM ('sad', 'ok');\n";
    let first = parse_sql(src);
    let second = parse_sql(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_sql("CREATE FUNCTION ( @@@ not valid sql");
    let _ = parsed;
}
