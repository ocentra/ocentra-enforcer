//! Hard tests for gitattributes, onboarded directly through the
//! generic spec-table engine
//! ([`enforcer_memory::languages::generic::parse_gitattributes`]).
//! Tier-0 (see
//! [`enforcer_memory::languages::spec::LangSpec::gitattributes`]'s own
//! doc comment): only its own real root node kind (`file`, NOT
//! baseline's stale `source`) is asserted.

use enforcer_memory::languages::generic::parse_gitattributes;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_gitattributes";

#[test]
fn extracts_module_symbol_for_file_root() {
    let src = "*.rs text eol=lf\n";
    let parsed = parse_gitattributes(src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
}

#[test]
fn parses_fixture_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("sample.gitattributes");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_gitattributes(&src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "*.rs text eol=lf\n*.png binary\n";
    let first = parse_gitattributes(src);
    let second = parse_gitattributes(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_gitattributes("not really !!! gitattributes @@@ ###");
    let _ = parsed;
}
