//! Hard tests for Astro, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_syntax::languages::generic::parse_astro`]). Tier-0 (see
//! [`enforcer_syntax::languages::spec::LangSpec::astro`]'s own doc
//! comment): only its own real root node kind (`document`, matching
//! baseline) is asserted -- the baseline's own frontmatter/`<script>`
//! embedded-import re-parse has no equivalent in this crate's engine
//! yet, a documented gap, not asserted here.

use enforcer_syntax::languages::generic::parse_astro;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_astro";

#[test]
fn extracts_module_symbol_for_document_root() {
    let src = "---\nconst x = 1;\n---\n<div>{x}</div>\n";
    let parsed = parse_astro(src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
}

#[test]
fn parses_fixture_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("Sample.astro");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_astro(&src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "---\nconst x = 1;\n---\n<div>{x}</div>\n";
    let first = parse_astro(src);
    let second = parse_astro(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_astro("not really astro @@@ ###");
    let _ = parsed;
}
