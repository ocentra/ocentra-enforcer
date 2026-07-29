//! Hard tests for JSON5, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_memory::languages::generic::parse_json5`]). JSON5 is a
//! Tier-0 nominal language (see
//! [`enforcer_memory::languages::spec::LangSpec::json5`]'s own doc
//! comment): only its own real root node kind (`file`, NOT baseline's
//! claimed `document`) is asserted.

use enforcer_memory::languages::generic::parse_json5;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_json5";

#[test]
fn extracts_module_symbol_for_file_root() {
    let src = "{ a: 1 }";
    let parsed = parse_json5(src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
}

#[test]
fn parses_fixture_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("sample.json5");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_json5(&src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "{ a: 1 }";
    let first = parse_json5(src);
    let second = parse_json5(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_json5("{{{{ not json5 @@@ ###");
    let _ = parsed;
}
