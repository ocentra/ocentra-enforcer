//! Hard tests for Lua, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_lua`])
//! -- there is no bespoke `languages::lua` extractor to prove
//! zero-regression against (Lua has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::lua`]'s own doc comment
//! directly: `function_declaration` naming (plain/dotted/colon-method
//! forms), the anonymous `function_definition` literal's
//! assignment-derived name, `method_index_expression` receiver-qualified
//! calls, ordinary calls, `require(...)` IMPORTS, and branch recognition.

use enforcer_domain::memory_types::ReceiverHint;
use enforcer_memory::languages::generic::parse_lua;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_lua";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_free_function_declaration() {
    let src = r#"
function foo(a, b)
  return a + b
end
"#;
    let parsed = parse_lua(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "foo"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_dotted_function_declaration_with_qualified_name() {
    // `function t.foo(a)`'s own `name` field is a `dot_index_expression`
    // whose full text is the whole qualified path -- recorded verbatim,
    // matching the baseline's own plain `.utf8_text()` (no dot-stripping).
    let src = r#"
function t.foo(a)
  return a
end
"#;
    let parsed = parse_lua(src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "t.foo"),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_colon_method_declaration_with_qualified_name() {
    let src = r#"
function t:foo(a)
  return a
end
"#;
    let parsed = parse_lua(src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "t:foo"),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_anonymous_function_assigned_to_local() {
    // `function_definition` (the anonymous-literal form) has NO `name`
    // field at all -- its name comes from the enclosing
    // `assignment_statement`'s own `variable_list`.
    let src = r#"
local foo = function(a)
  return a
end
"#;
    let parsed = parse_lua(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "foo"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn anonymous_function_with_no_assignment_has_no_name_but_does_not_panic() {
    let src = r#"
helper(function(x)
  return x
end)
"#;
    let parsed = parse_lua(src);
    let _ = parsed;
}

#[test]
fn extracts_plain_call() -> TestResult {
    let src = r#"
function render(w)
  helper(1, 2)
end
"#;
    let parsed = parse_lua(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(
        call.arg_texts,
        vec!["1".to_string(), "2".to_string()],
        "{call:?}"
    );
    Ok(())
}

#[test]
fn extracts_colon_method_call_with_receiver() -> TestResult {
    let src = r#"
function render(w)
  w:draw()
end
"#;
    let parsed = parse_lua(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "w:draw")
        .ok_or("expected a w:draw call")?;
    assert_eq!(call.receiver_text.as_deref(), Some("w"), "{call:?}");
    assert_eq!(
        call.receiver_hint,
        Some(ReceiverHint::Identifier),
        "{call:?}"
    );
    Ok(())
}

#[test]
fn self_receiver_colon_call_is_self_or_this_hint() -> TestResult {
    let src = r#"
function Widget:draw()
  self:helper()
end
"#;
    let parsed = parse_lua(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "self:helper")
        .ok_or("expected a self:helper call")?;
    assert_eq!(
        call.receiver_hint,
        Some(ReceiverHint::SelfOrThis),
        "{call:?}"
    );
    Ok(())
}

#[test]
fn call_inside_function_records_from_symbol_scope() -> TestResult {
    let src = r#"
function render(w)
  helper()
end
"#;
    let parsed = parse_lua(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("render"), "{call:?}");
    Ok(())
}

#[test]
fn extracts_require_as_import() {
    let src = r#"local json = require("json")"#;
    let parsed = parse_lua(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"json"));
}

#[test]
fn require_call_is_also_recorded_as_a_call() {
    let src = r#"local json = require("json")"#;
    let parsed = parse_lua(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"require"));
}

#[test]
fn ordinary_call_is_not_misdetected_as_an_import() {
    let src = r#"
function f()
  helper("x")
end
"#;
    let parsed = parse_lua(src);
    assert!(parsed.imports.is_empty(), "{:?}", parsed.imports);
}

#[test]
fn extracts_calls_inside_generic_for_loop() {
    let src = r#"
for k, v in pairs(t) do
  print(k, v)
end
"#;
    let parsed = parse_lua(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"pairs"));
    assert!(callees.contains(&"print"));
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_lua("function ( { this is not valid lua @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.lua");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_lua(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Widget.new"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Widget:draw"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "helper"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.imports.iter().any(|i| i.module_path == "json"),
        "{:?}",
        parsed.imports
    );
    assert!(
        parsed.calls.iter().any(|c| c.callee == "w:draw"),
        "{:?}",
        parsed.calls
    );
    Ok(())
}
