//! Hard tests for gitignore, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_memory::languages::generic::parse_gitignore`]) --
//! grammar VENDORED (`vendor/tree-sitter-gitignore-local/`). Matches
//! the baseline's own fully nominal row: a `.gitignore` file has no
//! func/class/call/import concept this crate's own [`LangSpec`] shape
//! models at all -- these tests assert only "parses without panicking"
//! plus the one real structural signal this crate can record, a
//! module symbol for the file's own `document` root.
//!
//! [`LangSpec`]: enforcer_memory::languages::spec::LangSpec

use enforcer_memory::languages::generic::parse_gitignore;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_gitignore";

#[test]
fn extracts_module_symbol_for_document_root() {
    let src = "node_modules/\n*.log\n";
    let parsed = parse_gitignore(src);
    assert!(
        parsed.symbols.iter().any(|s| s.kind == SymbolKind::Module),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn parses_fixture_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join(".gitignore");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_gitignore(&src);
    assert!(parsed.calls.is_empty(), "{:?}", parsed.calls);
    assert!(parsed.imports.is_empty(), "{:?}", parsed.imports);
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "*.log\n!important.log\n";
    let first = parse_gitignore(src);
    let second = parse_gitignore(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_gitignore("[[[not a valid pattern @@@");
    let _ = parsed;
}
