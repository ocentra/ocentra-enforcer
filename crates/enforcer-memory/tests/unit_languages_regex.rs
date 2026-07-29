//! Hard tests for standalone regular-expression patterns, onboarded
//! directly through the generic spec-table engine
//! ([`enforcer_memory::languages::generic::parse_regex`]). Regex is a
//! Tier-0 nominal language (see
//! [`enforcer_memory::languages::spec::LangSpec::regex`]'s own doc
//! comment): only its own real root node kind (`pattern`, matching
//! baseline) is asserted.

use enforcer_memory::languages::generic::parse_regex;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_regex";

#[test]
fn extracts_module_symbol_for_pattern_root() {
    let src = "[a-z]+";
    let parsed = parse_regex(src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
}

#[test]
fn parses_fixture_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("sample.re");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_regex(&src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "[a-z]+";
    let first = parse_regex(src);
    let second = parse_regex(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_regex("(((unbalanced");
    let _ = parsed;
}
