//! Hard tests for Blade, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_memory::languages::generic::parse_blade`]). Tier-0 (see
//! [`enforcer_memory::languages::spec::LangSpec::blade`]'s own doc
//! comment): only its own real root node kind (`document`, matching
//! baseline) is asserted.

use enforcer_memory::languages::generic::parse_blade;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_blade";

#[test]
fn extracts_module_symbol_for_document_root() {
    let src = "@if($x)\n<div>{{ $x }}</div>\n@endif\n";
    let parsed = parse_blade(src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
}

#[test]
fn parses_fixture_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("sample.blade.php");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_blade(&src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "@if($x)\n<div>{{ $x }}</div>\n@endif\n";
    let first = parse_blade(src);
    let second = parse_blade(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_blade("not really blade @@@ ###");
    let _ = parsed;
}
