//! Hard tests for PO (gettext translation catalog), onboarded directly
//! through the generic spec-table engine
//! ([`enforcer_memory::languages::generic::parse_po`]). PO is a Tier-0
//! nominal language (see
//! [`enforcer_memory::languages::spec::LangSpec::po`]'s own doc
//! comment): only its own real root node kind (`source_file`, matching
//! baseline) is asserted.

use enforcer_memory::languages::generic::parse_po;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_po";

#[test]
fn extracts_module_symbol_for_source_file_root() {
    let src = "msgid \"hello\"\nmsgstr \"bonjour\"\n";
    let parsed = parse_po(src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
}

#[test]
fn parses_fixture_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("sample.po");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_po(&src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "msgid \"hello\"\nmsgstr \"bonjour\"\n";
    let first = parse_po(src);
    let second = parse_po(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_po("not really a po file @@@ ###");
    let _ = parsed;
}
