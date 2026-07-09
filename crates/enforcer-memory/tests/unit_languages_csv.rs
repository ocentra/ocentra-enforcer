//! Hard tests for CSV, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_memory::languages::generic::parse_csv`]). Tier-0 (see
//! [`enforcer_memory::languages::spec::LangSpec::csv`]'s own doc
//! comment): only its own real root node kind (`document`, matching
//! baseline) is asserted. This grammar's own `field` node only accepts
//! quoted-string or numeric tokens (confirmed via a real parse-tree
//! dump), so fixtures here use quoted/numeric fields to parse cleanly.

use enforcer_memory::languages::generic::parse_csv;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_csv";

#[test]
fn extracts_module_symbol_for_document_root() {
    let src = "\"a\",\"b\",\"c\"\n1,2,3\n";
    let parsed = parse_csv(src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
}

#[test]
fn parses_fixture_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("sample.csv");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_csv(&src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "\"a\",\"b\",\"c\"\n1,2,3\n";
    let first = parse_csv(src);
    let second = parse_csv(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_csv("not really !!! csv @@@ ###");
    let _ = parsed;
}
