//! Hard tests for JSON, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_syntax::languages::generic::parse_json`]) -- grammar:
//! `tree-sitter-json` 0.24.8. Matches the baseline's own fully nominal
//! row (baseline's own `json_var_types: ["pair"]` has no equivalent in
//! this crate's own narrower [`LangSpec`] shape, see that row's own
//! doc comment) -- these tests assert only "parses without panicking"
//! plus the one real structural signal this crate can record, a
//! module symbol for the file's own `document` root, and that `.json`
//! now classifies to the real extractor rather than the pre-existing
//! `ConfigJson` no-op fallback.
//!
//! [`LangSpec`]: enforcer_syntax::languages::spec::LangSpec

use enforcer_syntax::parsers::SymbolKind;
use enforcer_syntax::{languages::generic::parse_json, parsers};
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_json";

#[test]
fn extracts_module_symbol_for_document_root() {
    let src = "{\"a\": 1}";
    let parsed = parse_json(src);
    assert!(
        parsed.symbols.iter().any(|s| s.kind == SymbolKind::Module),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn classify_routes_json_extension_to_real_extractor() {
    assert_eq!(parsers::classify("config.json"), parsers::Language::Json);
}

#[test]
fn parses_fixture_config_json_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("config.json");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_json(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.kind == SymbolKind::Module),
        "{:?}",
        parsed.symbols
    );
    assert!(parsed.calls.is_empty(), "{:?}", parsed.calls);
    assert!(parsed.imports.is_empty(), "{:?}", parsed.imports);
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "{\"a\": [1, 2, 3]}";
    let first = parse_json(src);
    let second = parse_json(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_json("{not valid json @@@");
    let _ = parsed;
}
