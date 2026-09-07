//! Hard tests for Julia, onboarded directly through the generic
//! spec-table engine ([`enforcer_syntax::languages::generic::parse_julia`])
//! -- there is no bespoke `languages::julia` extractor to prove
//! zero-regression against (Julia has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::julia`]'s own doc
//! comment directly: entirely unfielded function/struct/call nodes,
//! `type_head`'s `<:` supertype INHERITS, the short-form
//! `f(x) = body`-is-a-def-only-when-LHS-is-a-call gate, and
//! `module_definition`'s real `name` field.

use enforcer_domain::memory_types::ReceiverHint;
use enforcer_syntax::languages::generic::parse_julia;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_julia";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_plain_function_symbol() {
    let src = "function draw(w)\n    helper(w)\n    return 1\nend\n";
    let parsed = parse_julia(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "draw"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_bare_call_edge_inside_function_with_from_symbol_scope() -> TestResult {
    let src = "function draw(w)\n    helper(w)\nend\n";
    let parsed = parse_julia(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("draw"));
    Ok(())
}

#[test]
fn extracts_struct_with_supertype_as_inherits() {
    let src = "struct Dog <: Animal\n    name::String\nend\n";
    let parsed = parse_julia(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Dog"),
        Some(&SymbolKind::Struct),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed
            .inherits
            .iter()
            .any(|i| i.sub_name == "Dog" && i.super_name == "Animal"),
        "{:?}",
        parsed.inherits
    );
}

#[test]
fn plain_struct_has_no_inherits_edge() {
    let src = "struct Point\n    x::Int\n    y::Int\nend\n";
    let parsed = parse_julia(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Point"),
        Some(&SymbolKind::Struct)
    );
    assert!(parsed.inherits.is_empty(), "{:?}", parsed.inherits);
}

#[test]
fn extracts_abstract_type_with_supertype_as_class_and_inherits() {
    let src = "abstract type Shape <: Drawable end\n";
    let parsed = parse_julia(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Shape"),
        Some(&SymbolKind::Class)
    );
    assert!(
        parsed
            .inherits
            .iter()
            .any(|i| i.sub_name == "Shape" && i.super_name == "Drawable"),
        "{:?}",
        parsed.inherits
    );
}

#[test]
fn mutable_struct_extracts_same_as_plain_struct() {
    let src = "mutable struct Dog <: Animal\n    name::String\nend\n";
    let parsed = parse_julia(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Dog"),
        Some(&SymbolKind::Struct)
    );
    assert!(
        parsed
            .inherits
            .iter()
            .any(|i| i.sub_name == "Dog" && i.super_name == "Animal"),
        "{:?}",
        parsed.inherits
    );
}

#[test]
fn extracts_typed_return_signature_function_name() {
    let src = "function area(s::Shape)::Float64\n    return 0.0\nend\n";
    let parsed = parse_julia(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "area"),
        Some(&SymbolKind::Function)
    );
}

#[test]
fn extracts_dot_call_with_identifier_receiver() -> TestResult {
    let src = "function f()\n    obj.draw()\nend\n";
    let parsed = parse_julia(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "obj.draw")
        .ok_or("expected an obj.draw call")?;
    assert_eq!(call.receiver_text.as_deref(), Some("obj"));
    assert_eq!(call.receiver_hint, Some(ReceiverHint::Identifier));
    Ok(())
}

#[test]
fn extracts_module_qualified_call() -> TestResult {
    let src = "function f()\n    Base.show(io, x)\nend\n";
    let parsed = parse_julia(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "Base.show")
        .ok_or("expected a Base.show call")?;
    assert_eq!(
        call.arg_texts,
        vec!["io".to_string(), "x".to_string()],
        "{call:?}"
    );
    Ok(())
}

#[test]
fn extracts_broadcast_call_edge() -> TestResult {
    let src = "function f()\n    map.(x, y)\nend\n";
    let parsed = parse_julia(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "map")
        .ok_or("expected a map broadcast call")?;
    assert_eq!(
        call.arg_texts,
        vec!["x".to_string(), "y".to_string()],
        "{call:?}"
    );
    Ok(())
}

#[test]
fn short_form_function_with_call_lhs_is_a_function_def() {
    let src = "square(x) = x * x\n";
    let parsed = parse_julia(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "square"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn short_form_function_call_in_rhs_records_from_symbol_scope() -> TestResult {
    let src = "square(x) = helper(x)\n";
    let parsed = parse_julia(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("square"));
    Ok(())
}

#[test]
fn short_form_function_does_not_record_spurious_self_call() {
    // The LHS `square(x)` must never itself surface as a CALLS edge (it
    // is the function's own signature, not an invocation) -- see
    // `LangSpec::julia`'s own doc comment for the exact bug this guards.
    let src = "square(x) = x * x\n";
    let parsed = parse_julia(src);
    assert!(
        !parsed.calls.iter().any(|c| c.callee == "square"),
        "{:?}",
        parsed.calls
    );
}

#[test]
fn plain_assignment_with_non_call_lhs_is_not_a_function_def() {
    let src = "x = 5\n";
    let parsed = parse_julia(src);
    assert!(
        symbol_kind(&parsed.symbols, "x").is_none(),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn plain_assignment_rhs_call_is_still_found() -> TestResult {
    let src = "x = helper()\n";
    let parsed = parse_julia(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol, None);
    Ok(())
}

#[test]
fn extracts_using_statement_as_import() {
    let src = "using Base: show\n";
    let parsed = parse_julia(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"Base"));
}

#[test]
fn extracts_import_statement_as_import() {
    let src = "import Foo\n";
    let parsed = parse_julia(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"Foo"));
}

#[test]
fn extracts_export_statement_names_as_imports() {
    let src = "export draw, area\n";
    let parsed = parse_julia(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"draw"));
    assert!(paths.contains(&"area"));
}

#[test]
fn extracts_module_definition_as_module_symbol() {
    let src = "module MyMod\nfunction f() end\nend\n";
    let parsed = parse_julia(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "MyMod"),
        Some(&SymbolKind::Module),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn nested_function_inside_function_is_found_as_closure() {
    let src = "function outer()\n    function inner()\n        return 1\n    end\n    return inner()\nend\n";
    let parsed = parse_julia(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "outer"),
        Some(&SymbolKind::Function)
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "inner"),
        Some(&SymbolKind::Function)
    );
}

#[test]
fn extracts_branch_heavy_function_without_panicking() {
    let src = r#"
function render()
    if square(2) > 0
        helper(1, 2)
    else
        helper(0, 0)
    end
    for i in 1:10
        helper(i)
    end
    while true
        break
    end
    try
        helper(1)
    catch e
        helper(2)
    end
end
"#;
    let parsed = parse_julia(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "render"),
        Some(&SymbolKind::Function)
    );
}

#[test]
fn parses_fixture_widget_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("widget.jl");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_julia(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Widgets"),
        Some(&SymbolKind::Module),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Dog"),
        Some(&SymbolKind::Struct)
    );
    assert!(
        parsed
            .inherits
            .iter()
            .any(|i| i.sub_name == "Dog" && i.super_name == "Shape"),
        "{:?}",
        parsed.inherits
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "draw"),
        Some(&SymbolKind::Function)
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "square"),
        Some(&SymbolKind::Function)
    );
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "function draw(w)\n    helper(w)\nend\n";
    let first = parse_julia(src);
    let second = parse_julia(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_julia("function ( { this is not valid julia @@@");
    let _ = parsed;
}
