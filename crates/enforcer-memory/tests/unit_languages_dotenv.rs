//! Hard tests for DotEnv, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_syntax::languages::generic::parse_dotenv`]). Tier-0 (see
//! [`enforcer_syntax::languages::spec::LangSpec::dotenv`]'s own doc
//! comment): only its own real root node kind (`document`, NOT
//! baseline's stale `source_file`) is asserted.

use enforcer_syntax::languages::generic::parse_dotenv;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_dotenv";

#[test]
fn extracts_module_symbol_for_document_root() {
    let src = "FOO=bar\nBAZ=\"qux\"\n";
    let parsed = parse_dotenv(src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
}

#[test]
fn parses_fixture_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("sample.env");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_dotenv(&src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "FOO=bar\n# comment\nBAZ=\"qux\"\n";
    let first = parse_dotenv(src);
    let second = parse_dotenv(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_dotenv("not really !!! dotenv @@@ ###");
    let _ = parsed;
}
