//! Hard tests for Beancount, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_memory::languages::generic::parse_beancount`]). Tier-0
//! (see [`enforcer_memory::languages::spec::LangSpec::beancount`]'s own
//! doc comment): `include`'s sole `string` child is fieldless, so
//! [`enforcer_memory::languages::generic::beancount_quirk`] reads it
//! positionally, quotes stripped.

use enforcer_memory::languages::generic::parse_beancount;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_beancount";

#[test]
fn extracts_module_symbol_for_file_root() {
    let src = "2020-01-01 open Assets:Cash\n";
    let parsed = parse_beancount(src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
}

#[test]
fn extracts_include_as_import_with_quotes_stripped() {
    let src = "include \"other.beancount\"\n";
    let parsed = parse_beancount(src);
    assert!(
        parsed
            .imports
            .iter()
            .any(|i| i.module_path == "other.beancount"),
        "{:?}",
        parsed.imports
    );
}

#[test]
fn parses_fixture_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("sample.beancount");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_beancount(&src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
    assert!(
        parsed
            .imports
            .iter()
            .any(|i| i.module_path == "other.beancount"),
        "{:?}",
        parsed.imports
    );
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "include \"other.beancount\"\n2020-01-01 open Assets:Cash\n";
    let first = parse_beancount(src);
    let second = parse_beancount(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_beancount("not really beancount @@@ ###");
    let _ = parsed;
}
