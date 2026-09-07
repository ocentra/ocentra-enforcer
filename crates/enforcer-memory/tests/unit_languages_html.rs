//! Hard tests for HTML, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_syntax::languages::generic::parse_html`]) -- grammar:
//! `tree-sitter-html` 0.23.2. Matches the baseline's own fully nominal
//! row: no func/class/call/import concept modeled here at all, and the
//! baseline's own `<script>`-embedded-JS-import reparse is explicitly
//! DEFERRED (see [`enforcer_syntax::languages::generic::parse_html`]'s
//! own doc comment) -- these tests assert only "parses without
//! panicking" plus the one real structural signal this crate can
//! record, a module symbol for the file's own `document` root.

use enforcer_syntax::parsers::SymbolKind;
use enforcer_syntax::{languages::generic::parse_html, parsers};
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_html";

#[test]
fn extracts_module_symbol_for_document_root() {
    let src = "<html><body></body></html>";
    let parsed = parse_html(src);
    assert!(
        parsed.symbols.iter().any(|s| s.kind == SymbolKind::Module),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn embedded_script_content_is_not_extracted_as_import() {
    let src = "<script>import x from './x.js';</script>";
    let parsed = parse_html(src);
    assert!(parsed.imports.is_empty(), "{:?}", parsed.imports);
}

#[test]
fn parses_fixture_page_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("page.html");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_html(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.kind == SymbolKind::Module),
        "{:?}",
        parsed.symbols
    );
    Ok(())
}

#[test]
fn classify_routes_html_and_htm_extensions() {
    assert_eq!(parsers::classify("index.html"), parsers::Language::Html);
    assert_eq!(parsers::classify("index.htm"), parsers::Language::Html);
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "<html><head></head><body></body></html>";
    let first = parse_html(src);
    let second = parse_html(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_html("<html <<< not valid @@@");
    let _ = parsed;
}
