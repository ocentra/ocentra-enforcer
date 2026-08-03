//! Hard tests for CFML (tag dialect), onboarded directly through the
//! generic spec-table engine
//! ([`enforcer_syntax::languages::generic::parse_cfml`]) -- there is no
//! bespoke `languages::cfml` extractor to prove zero-regression against
//! (CFML has never had one in this crate), so these tests assert
//! against the grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::cfml`]'s own doc
//! comment directly: `cf_function_tag`'s fully fieldless name
//! resolution via `cf_attribute`/`cf_attribute_name`/
//! `quoted_cf_attribute_value` descent, and real `call_expression`
//! `function`/`arguments` fields.

use enforcer_syntax::languages::generic::parse_cfml;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_cfml";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_cffunction_tag_name_via_quirk() {
    let src = r#"<cfcomponent>
<cffunction name="getUser" access="public">
    <cfreturn 1>
</cffunction>
</cfcomponent>
"#;
    let parsed = parse_cfml(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "getUser"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_cffunction_tag_name_case_insensitively() {
    let src = r#"<cfcomponent>
<cffunction NAME="getUser">
    <cfreturn 1>
</cffunction>
</cfcomponent>
"#;
    let parsed = parse_cfml(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "getUser"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_script_mode_function_declaration_via_real_name_field() {
    let src = "<cfscript>\nfunction getUser(id) {\n    return id;\n}\n</cfscript>\n";
    let parsed = parse_cfml(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "getUser"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_call_inside_cffunction_tag() -> TestResult {
    let src = r#"<cfcomponent>
<cffunction name="getUser">
    <cfset logAccess(1)>
</cffunction>
</cfcomponent>
"#;
    let parsed = parse_cfml(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "logAccess")
        .ok_or("expected a logAccess call")?;
    let _ = call;
    Ok(())
}

#[test]
fn parses_fixture_user_component_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("UserComponent.cfm");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_cfml(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "getUser"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.calls.iter().any(|c| c.callee == "logAccess"),
        "{:?}",
        parsed.calls
    );
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = r#"<cfcomponent>
<cffunction name="getUser">
    <cfreturn 1>
</cffunction>
</cfcomponent>
"#;
    let first = parse_cfml(src);
    let second = parse_cfml(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_cfml("<cffunction name=@@@ this is not valid cfml <<<");
    let _ = parsed;
}
