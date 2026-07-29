//! Hard tests for Janet, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_memory::languages::generic::parse_janet`]) -- grammar
//! VENDORED (`vendor/tree-sitter-janet-local/`; the grammar's own
//! generated C function is `tree_sitter_janet_simple`). Matches the
//! baseline's own fully nominal row: every list-shaped Janet form is
//! the SAME completely fieldless tuple-literal node kind regardless of
//! its own head symbol, so these tests assert only "parses without
//! panicking" plus the one real structural signal this crate can
//! record, a module symbol for the file's own `source` root.

use enforcer_memory::languages::generic::parse_janet;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_janet";

#[test]
fn extracts_module_symbol_for_source_root() {
    let src = "(print \"hi\")\n";
    let parsed = parse_janet(src);
    assert!(
        parsed.symbols.iter().any(|s| s.kind == SymbolKind::Module),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn parses_fixture_script_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("script.janet");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_janet(&src);
    assert!(parsed.calls.is_empty(), "{:?}", parsed.calls);
    assert!(
        parsed.symbols.iter().any(|s| s.kind == SymbolKind::Module),
        "{:?}",
        parsed.symbols
    );
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "(defn foo [x] (+ x 1))\n";
    let first = parse_janet(src);
    let second = parse_janet(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_janet("(((not valid @@@");
    let _ = parsed;
}
