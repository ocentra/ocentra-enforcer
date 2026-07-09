//! Hard tests for CSS, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_memory::languages::generic::parse_css`]). Tier-0 (see
//! [`enforcer_memory::languages::spec::LangSpec::css`]'s own doc
//! comment): `call_expression`/`import_statement` are both fully
//! fieldless (confirmed via a real `node-types.json` dump), so
//! [`enforcer_memory::languages::generic::css_call_override`]/
//! [`enforcer_memory::languages::generic::css_import_quirk`] both read
//! their content positionally.

use enforcer_memory::languages::generic::parse_css;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_css";

#[test]
fn extracts_module_symbol_for_stylesheet_root() {
    let src = ".a { color: red; }\n";
    let parsed = parse_css(src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
}

#[test]
fn extracts_import_statement_path() {
    let src = "@import \"foo.css\";\n";
    let parsed = parse_css(src);
    assert!(
        parsed.imports.iter().any(|i| i.module_path == "foo.css"),
        "{:?}",
        parsed.imports
    );
}

#[test]
fn extracts_call_expression_via_positional_function_name() {
    let src = ".a { width: calc(100% - 10px); }\n";
    let parsed = parse_css(src);
    assert!(
        parsed.calls.iter().any(|c| c.callee == "calc"),
        "{:?}",
        parsed.calls
    );
}

#[test]
fn parses_fixture_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("sample.css");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_css(&src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
    assert!(
        parsed.imports.iter().any(|i| i.module_path == "foo.css"),
        "{:?}",
        parsed.imports
    );
    assert!(
        parsed.calls.iter().any(|c| c.callee == "calc"),
        "{:?}",
        parsed.calls
    );
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "@import \"foo.css\";\n.a { color: red; }\n";
    let first = parse_css(src);
    let second = parse_css(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_css("not really css @@@ ###");
    let _ = parsed;
}
