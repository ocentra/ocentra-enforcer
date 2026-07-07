//! Hard tests for F#, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_fsharp`])
//! -- there is no bespoke `languages::fsharp` extractor to prove
//! zero-regression against (F# has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::fsharp`]'s own doc
//! comment directly: `function_or_value_defn` signature/body-field
//! split naming, `anon_type_defn`/`record_type_defn`/`union_type_defn`
//! positional `type_name` naming + `inherit Base(...)` INHERITS,
//! `named_module`/`namespace` dot-joined naming, `import_decl` IMPORTS,
//! and `application_expression`'s narrow (baseline-matching)
//! curried-call callee-head reconstruction.

use enforcer_memory::languages::generic::parse_fsharp;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_fsharp";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_free_function_from_let_binding() {
    let src = r#"
let helper x =
    printfn "helper"
"#;
    let parsed = parse_fsharp(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_class_with_primary_constructor_as_anon_type_defn() {
    // `type Animal(name: string) = ...` parses as `anon_type_defn`, NOT
    // one of the other four `_type_defn_body` alternatives -- caught
    // only by a real parse tree dump, not by reading node-types.json
    // alone (see `LangSpec::fsharp`'s own doc comment).
    let src = r#"
type Animal(name: string) =
    member this.Speak() = printfn "..."
"#;
    let parsed = parse_fsharp(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Animal"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_inherit_as_inherits_edge() {
    let src = r#"
type Widget(name: string) =
    inherit Animal(name)
"#;
    let parsed = parse_fsharp(src);
    let inherits: Vec<(&str, &str)> = parsed
        .inherits
        .iter()
        .map(|i| (i.sub_name.as_str(), i.super_name.as_str()))
        .collect();
    assert!(inherits.contains(&("Widget", "Animal")), "{inherits:?}");
}

#[test]
fn extracts_union_type_defn_as_class() {
    let src = r#"
type Shape =
    | Circle of float
    | Square of float
"#;
    let parsed = parse_fsharp(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Shape"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_record_type_defn_as_class() {
    let src = r#"
type Point = { X: float; Y: float }
"#;
    let parsed = parse_fsharp(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Point"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_type_abbrev_defn_as_type_alias_for_function_type_rhs() {
    // A bare single-identifier RHS (`type Meters = float`) is a genuine
    // grammar ambiguity that resolves to `union_type_defn` instead (see
    // `LangSpec::fsharp`'s own doc comment) -- a function-type RHS does
    // not hit that ambiguity and correctly parses as `type_abbrev_defn`.
    let src = r#"
type Handler = int -> string
"#;
    let parsed = parse_fsharp(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Handler"),
        Some(&SymbolKind::TypeAlias),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_namespace_as_module() {
    let src = r#"
namespace Widgets

let x = 1
"#;
    let parsed = parse_fsharp(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Widgets"),
        Some(&SymbolKind::Module),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_named_module_dot_joined_name() {
    let src = r#"
module Widgets.Core

let x = 1
"#;
    let parsed = parse_fsharp(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Widgets.Core"),
        Some(&SymbolKind::Module),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_open_as_imports_edge() {
    let src = r#"
open System.Text
"#;
    let parsed = parse_fsharp(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"System.Text"), "{paths:?}");
}

#[test]
fn extracts_simple_application_call() -> TestResult {
    let src = r#"
let draw x =
    helper x
"#;
    let parsed = parse_fsharp(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.arg_texts, vec!["x".to_string()], "{call:?}");
    Ok(())
}

#[test]
fn zero_arg_application_call_has_empty_arg_texts() -> TestResult {
    let src = r#"
let draw () =
    helper()
"#;
    let parsed = parse_fsharp(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert!(call.arg_texts.is_empty(), "{call:?}");
    Ok(())
}

#[test]
fn curried_multi_arg_call_only_records_the_narrow_inner_application() {
    // Mirrors `internal/cbm/extract_calls.c`'s own `extract_fsharp_callee`
    // real (narrow) depth exactly: `add x y` parses as nested
    // `application_expression(application_expression(add, x), y)`, and
    // the OUTER node's own head is the inner application (not an
    // identifier), so no CALLS edge is ever recorded for the full `add x
    // y` shape -- only the inner `add x` application resolves to a
    // callee.
    let src = r#"
let draw x y =
    add x y
"#;
    let parsed = parse_fsharp(src);
    let add_calls: Vec<_> = parsed.calls.iter().filter(|c| c.callee == "add").collect();
    assert_eq!(add_calls.len(), 1, "{:?}", parsed.calls);
    assert_eq!(
        add_calls[0].arg_texts,
        vec!["x".to_string()],
        "{:?}",
        add_calls[0]
    );
}

#[test]
fn call_inside_let_binding_records_from_symbol_scope() -> TestResult {
    let src = r#"
let draw x =
    helper x
"#;
    let parsed = parse_fsharp(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("draw"), "{call:?}");
    Ok(())
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_fsharp("let ( { this is not valid fsharp @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("Widget.fs");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_fsharp(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Widgets"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Widget"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "helper"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed
            .imports
            .iter()
            .any(|i| i.module_path == "System.Text"),
        "{:?}",
        parsed.imports
    );
    assert!(
        parsed
            .inherits
            .iter()
            .any(|i| i.sub_name == "Widget" && i.super_name == "Animal"),
        "{:?}",
        parsed.inherits
    );
    Ok(())
}
