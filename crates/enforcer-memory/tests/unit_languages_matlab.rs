//! Hard tests for MATLAB, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_memory::languages::generic::parse_matlab`]) -- there is no
//! bespoke `languages::matlab` extractor to prove zero-regression
//! against, so these tests assert against the grammar-shape ground truth
//! recorded in [`enforcer_memory::languages::spec::LangSpec::matlab`]'s
//! own doc comment directly: `function_definition`'s quirk-claimed
//! name/body walk, `class_definition` DEFINES nesting, `function_call`
//! vs. unparenthesized `command`-syntax calls, and branch recognition.

use enforcer_memory::languages::generic::parse_matlab;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_matlab";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_free_function_definition() {
    let src = r#"
function out = foo(a, b)
  out = a + b;
end
"#;
    let parsed = parse_matlab(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "foo"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_class_definition() {
    let src = r#"
classdef Widget
  properties
    name
  end
end
"#;
    let parsed = parse_matlab(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Widget"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn method_inside_classdef_records_defines_edge() {
    let src = r#"
classdef Widget
  methods
    function obj = draw(obj)
      obj = helper(obj);
    end
  end
end
"#;
    let parsed = parse_matlab(src);
    assert!(
        parsed
            .defines
            .iter()
            .any(|d| d.container_name == "Widget" && d.member_name == "draw"),
        "{:?}",
        parsed.defines
    );
}

#[test]
fn call_inside_function_is_recorded_with_from_symbol_scope() -> TestResult {
    let src = r#"
function out = foo(a)
  out = helper(a);
end
"#;
    let parsed = parse_matlab(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("foo"), "{call:?}");
    Ok(())
}

#[test]
fn extracts_unparenthesized_command_syntax_call() {
    let src = r#"
function foo()
  close all
end
"#;
    let parsed = parse_matlab(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"close"), "{callees:?}");
}

#[test]
fn extracts_calls_inside_if_branch() {
    let src = r#"
function foo(x)
  if x
    helper();
  end
end
"#;
    let parsed = parse_matlab(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"helper"), "{callees:?}");
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_matlab("function ( { this is not valid matlab @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.m");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_matlab(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Widget"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "draw"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "helper"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.calls.iter().any(|c| c.callee == "close"),
        "{:?}",
        parsed.calls
    );
    Ok(())
}
