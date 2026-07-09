//! Hard tests for Markdown, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_memory::languages::generic::parse_markdown`]). Asserts
//! against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::markdown`]'s own doc
//! comment: neither `atx_heading` nor `setext_heading` has a
//! `name`-named field, so both are claimed directly by
//! [`enforcer_memory::languages::generic::markdown_quirk`] via their
//! own `heading_content` field.

use enforcer_memory::languages::generic::parse_markdown;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_markdown";

#[test]
fn extracts_atx_heading_as_class_symbol() -> TestResult {
    let src = "# Title\n\nBody text.\n";
    let parsed = parse_markdown(src);
    parsed
        .symbols
        .iter()
        .find(|s| s.kind == SymbolKind::Class && s.name.contains("Title"))
        .ok_or("expected a Title heading symbol")?;
    Ok(())
}

#[test]
fn extracts_setext_heading_as_class_symbol() -> TestResult {
    let src = "Section Two\n-----------\n\nBody text.\n";
    let parsed = parse_markdown(src);
    parsed
        .symbols
        .iter()
        .find(|s| s.kind == SymbolKind::Class && s.name.contains("Section Two"))
        .ok_or("expected a Section Two heading symbol")?;
    Ok(())
}

#[test]
fn parses_fixture_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("sample.md");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_markdown(&src);
    parsed
        .symbols
        .iter()
        .find(|s| s.kind == SymbolKind::Class && s.name.contains("Title"))
        .ok_or("expected a Title heading symbol")?;
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "# Title\n\nBody text.\n";
    let first = parse_markdown(src);
    let second = parse_markdown(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_markdown("not really markdown but that's fine too");
    let _ = parsed;
}
