//! Hard tests for Jinja2, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_memory::languages::generic::parse_jinja2`]) -- grammar:
//! `tree-sitter-jinja2` 0.0.16. Matches the baseline's own fully
//! nominal row: no func/class/call/import concept modeled here at all
//! -- these tests assert only "parses without panicking" plus the one
//! real structural signal this crate can record, a module symbol for
//! the file's own `source_file` root.

use enforcer_memory::languages::generic::parse_jinja2;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_jinja2";

#[test]
fn extracts_module_symbol_for_source_file_root() {
    let src = "{{ x }}\n";
    let parsed = parse_jinja2(src);
    assert!(
        parsed.symbols.iter().any(|s| s.kind == SymbolKind::Module),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn parses_for_loop_block_without_panicking() {
    let src = "{% for item in items %}{{ item }}{% endfor %}";
    let parsed = parse_jinja2(src);
    assert!(parsed.calls.is_empty(), "{:?}", parsed.calls);
}

#[test]
fn parses_fixture_template_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("template.jinja2");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_jinja2(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.kind == SymbolKind::Module),
        "{:?}",
        parsed.symbols
    );
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "{% if x %}{{ x }}{% endif %}";
    let first = parse_jinja2(src);
    let second = parse_jinja2(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_jinja2("{% not valid @@@");
    let _ = parsed;
}
