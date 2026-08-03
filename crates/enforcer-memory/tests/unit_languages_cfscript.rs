//! Hard tests for CFScript, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_syntax::languages::generic::parse_cfscript`]) -- there is
//! no bespoke `languages::cfscript` extractor to prove zero-regression
//! against (CFScript has never had one in this crate), so these tests
//! assert against the grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::cfscript`]'s own doc
//! comment directly: real `function_declaration` name/body fields
//! reused from the JS arrays, and the `tag_statement`-with-`tag`=
//! `"property"` quirk (this grammar has no dedicated field-declaration
//! node kind for CFScript at all).

use enforcer_syntax::languages::generic::parse_cfscript;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_cfscript";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_function_declaration_via_real_name_field() {
    let src = "component {\n    function getUser(string id) {\n        return id;\n    }\n}\n";
    let parsed = parse_cfscript(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "getUser"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_property_tag_statement_as_constant_via_quirk() {
    // Regression guard for the confirmed-dead baseline `field_types`
    // entry (see `LangSpec::cfscript`'s own doc comment): without
    // `cfscript_quirk`, this property declaration would be silently
    // invisible (this grammar has NO `property_declaration` node kind at
    // all -- it parses as an ordinary `tag_statement`).
    let src = "component {\n    property name=\"userId\" type=\"string\";\n}\n";
    let parsed = parse_cfscript(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "userId"),
        Some(&SymbolKind::Constant),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_call_inside_function_with_from_symbol_scope() -> TestResult {
    let src =
        "component {\n    function getUser(string id) {\n        return queryUser(id);\n    }\n}\n";
    let parsed = parse_cfscript(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "queryUser")
        .ok_or("expected a queryUser call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("getUser"));
    Ok(())
}

#[test]
fn extracts_import_statement() -> TestResult {
    let src = "import \"foo\";\n";
    let parsed = parse_cfscript(src);
    assert!(!parsed.imports.is_empty(), "{:?}", parsed.imports);
    Ok(())
}

#[test]
fn extracts_branch_heavy_function_without_panicking() {
    let src = "component {\n    function getUser(string id) {\n        if (id) {\n            return id;\n        } else {\n            return \"\";\n        }\n    }\n}\n";
    let parsed = parse_cfscript(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "getUser"),
        Some(&SymbolKind::Function)
    );
}

#[test]
fn parses_fixture_user_service_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("UserService.cfc");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_cfscript(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "getUser"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "queryUser"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "userId"),
        Some(&SymbolKind::Constant),
        "{:?}",
        parsed.symbols
    );
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "component {\n    function getUser() {\n        return 1;\n    }\n}\n";
    let first = parse_cfscript(src);
    let second = parse_cfscript(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_cfscript("function ( { this is not valid cfscript @@@");
    let _ = parsed;
}
