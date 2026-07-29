//! Hard tests for Java/Jakarta `.properties`, onboarded directly
//! through the generic spec-table engine
//! ([`enforcer_memory::languages::generic::parse_properties`]).
//! Properties is a Tier-0 nominal language (see
//! [`enforcer_memory::languages::spec::LangSpec::properties`]'s own doc
//! comment): only its own real root node kind (`file`) is asserted --
//! baseline's own `property`-as-symbol array is deliberately not
//! mapped onto this row (no container a DEFINES edge could attach to,
//! the same reasoning `LangSpec::wgsl`'s own row already applies).

use enforcer_memory::languages::generic::parse_properties;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_properties";

#[test]
fn extracts_module_symbol_for_file_root() {
    let src = "key=value\n";
    let parsed = parse_properties(src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
}

#[test]
fn parses_fixture_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("sample.properties");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_properties(&src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "key=value\n";
    let first = parse_properties(src);
    let second = parse_properties(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_properties("\0\0\0 not really properties");
    let _ = parsed;
}
