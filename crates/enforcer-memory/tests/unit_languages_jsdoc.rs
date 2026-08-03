//! Hard tests for JSDoc (standalone comment body), onboarded directly
//! through the generic spec-table engine
//! ([`enforcer_syntax::languages::generic::parse_jsdoc`]) -- grammar:
//! `tree-sitter-jsdoc` 0.25.0. No [`enforcer_syntax::parsers::classify`]
//! extension wiring at all (no baseline `EXT_TABLE` entry exists for
//! this language either) -- reached only via direct calls to
//! `parse_jsdoc`, matching the baseline's own fully nominal row
//! otherwise: these tests assert only "parses without panicking" plus
//! the one real structural signal this crate can record, a module
//! symbol for the comment's own `document` root.

use enforcer_syntax::languages::generic::parse_jsdoc;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_jsdoc";

#[test]
fn extracts_module_symbol_for_document_root() {
    let src = "/** Does a thing. */";
    let parsed = parse_jsdoc(src);
    assert!(
        parsed.symbols.iter().any(|s| s.kind == SymbolKind::Module),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn parses_fixture_comment_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("comment.jsdoc");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_jsdoc(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.kind == SymbolKind::Module),
        "{:?}",
        parsed.symbols
    );
    assert!(parsed.calls.is_empty(), "{:?}", parsed.calls);
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "/**\n * @param {string} foo desc\n */";
    let first = parse_jsdoc(src);
    let second = parse_jsdoc(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_jsdoc("not a comment at all @@@");
    let _ = parsed;
}
