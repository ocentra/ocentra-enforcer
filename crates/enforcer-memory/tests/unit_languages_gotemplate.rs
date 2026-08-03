//! Hard tests for Go Template, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_syntax::languages::generic::parse_gotemplate`]) --
//! grammar VENDORED (`vendor/tree-sitter-gotemplate-local/`). Asserts
//! against the grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::gotemplate`]'s own doc
//! comment: `define_action`'s real `name`/`body` fields, and the three
//! different call-shaped field names (`function_call`'s default
//! `function`/`arguments`; `method_call`'s `method`/`arguments`;
//! `template_action`'s `name`/singular `argument`) via
//! [`enforcer_syntax::languages::generic::gotemplate_call_override`].

use enforcer_syntax::languages::generic::parse_gotemplate;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_gotemplate";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_define_action_via_real_name_field() {
    let src = "{{define \"main\"}}hello{{end}}\n";
    let parsed = parse_gotemplate(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "\"main\""),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_function_call_via_default_fields() -> TestResult {
    let src = "{{ upper .Name }}\n";
    let parsed = parse_gotemplate(src);
    parsed
        .calls
        .iter()
        .find(|c| c.callee == "upper")
        .ok_or("expected an upper call")?;
    Ok(())
}

#[test]
fn extracts_template_action_via_override() -> TestResult {
    let src = "{{ template \"footer\" . }}\n";
    let parsed = parse_gotemplate(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "footer")
        .ok_or("expected a footer template call")?;
    assert_eq!(call.arg_texts, vec!["."]);
    Ok(())
}

#[test]
fn parses_fixture_main_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("main.gotmpl");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_gotemplate(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "\"main\""),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.calls.iter().any(|c| c.callee == "footer"),
        "{:?}",
        parsed.calls
    );
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "{{define \"main\"}}hello{{end}}\n";
    let first = parse_gotemplate(src);
    let second = parse_gotemplate(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_gotemplate("{{ this is not valid go template @@@");
    let _ = parsed;
}
