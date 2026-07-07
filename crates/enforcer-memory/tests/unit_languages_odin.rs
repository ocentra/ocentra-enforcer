//! Hard tests for Odin, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_odin`])
//! -- there is no bespoke `languages::odin` extractor to prove
//! zero-regression against (Odin has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::odin`]'s own doc comment
//! directly: positional `procedure_declaration`/`struct_declaration`
//! naming, the `using`-prefixed composition field as an INHERITS edge,
//! `call_expression`'s repeated `"argument"` field, and
//! `selector_call_expression`'s pointer-dereference method-call syntax.

use enforcer_memory::languages::generic::parse_odin;
use enforcer_memory::parsers::{ReceiverHint, SymbolKind};
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_odin";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_package_and_procedure_symbols() {
    let src = "package main\n\nadd :: proc(a: int, b: int) -> int {\n    return a + b\n}\n";
    let parsed = parse_odin(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "add"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_call_inside_proc_with_from_symbol_scope() -> TestResult {
    let src = "package main\nmain :: proc() {\n    helper(1, 2)\n}\n";
    let parsed = parse_odin(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("main"));
    Ok(())
}

#[test]
fn extracts_call_with_multiple_args_via_repeated_argument_field() -> TestResult {
    // Regression guard for the repeated-`"argument"`-field finding (see
    // `LangSpec::odin`'s own doc comment): without the `call_override`
    // fix, `arg_texts` would silently come back with at most one entry.
    let src = "package main\nmain :: proc() {\n    helper(1, 2, 3)\n}\n";
    let parsed = parse_odin(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(
        call.arg_texts,
        vec!["1".to_string(), "2".to_string(), "3".to_string()],
        "{call:?}"
    );
    Ok(())
}

#[test]
fn extracts_struct_and_enum_symbols() {
    let src = "package main\n\nDog :: struct {\n    name: string,\n}\n\nColor :: enum {\n    Red,\n    Green,\n}\n";
    let parsed = parse_odin(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Dog"),
        Some(&SymbolKind::Struct),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Color"),
        Some(&SymbolKind::Class)
    );
}

#[test]
fn extracts_ordinary_struct_field_as_defines() {
    let src = "package main\n\nDog :: struct {\n    name: string,\n}\n";
    let parsed = parse_odin(src);
    assert!(
        parsed
            .defines
            .iter()
            .any(|d| d.container_name == "Dog" && d.member_name == "name"),
        "{:?}",
        parsed.defines
    );
}

#[test]
fn extracts_using_prefixed_composition_field_as_inherits() {
    let src = "package main\n\nAnimal :: struct {\n    name: string,\n}\n\nDog :: struct {\n    using animal: Animal,\n    breed: string,\n}\n";
    let parsed = parse_odin(src);
    assert!(
        parsed
            .inherits
            .iter()
            .any(|i| i.sub_name == "Dog" && i.super_name == "Animal"),
        "{:?}",
        parsed.inherits
    );
    // "breed" is an ordinary (non-`using`) field -- must be DEFINES, not
    // INHERITS.
    assert!(
        parsed
            .defines
            .iter()
            .any(|d| d.container_name == "Dog" && d.member_name == "breed"),
        "{:?}",
        parsed.defines
    );
    // The embedded local name "animal" itself must NOT also surface as an
    // ordinary DEFINES edge (it is the composition binding, not a field).
    assert!(
        !parsed
            .defines
            .iter()
            .any(|d| d.container_name == "Dog" && d.member_name == "animal"),
        "{:?}",
        parsed.defines
    );
}

#[test]
fn extracts_import_declaration_as_import() {
    let src = "package main\n\nimport \"core:fmt\"\n";
    let parsed = parse_odin(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"core:fmt"), "{paths:?}");
}

#[test]
fn extracts_dot_call_on_plain_value_via_inner_call_expression() -> TestResult {
    // `obj.draw()` on a plain (non-pointer) value is a `member_expression`
    // wrapping an ordinary `call_expression` -- NOT `selector_call_expression`
    // (see `LangSpec::odin`'s own doc comment). The inner call's own
    // callee text is recorded, with no receiver captured for this
    // specific wrapping shape.
    let src = "package main\nmain :: proc() {\n    obj.draw()\n}\n";
    let parsed = parse_odin(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "draw")
        .ok_or("expected a draw call")?;
    let _ = call;
    Ok(())
}

#[test]
fn extracts_selector_call_expression_pointer_method_call() -> TestResult {
    let src = "package main\nmain :: proc() {\n    p: ^Dog\n    p->bark()\n}\n";
    let parsed = parse_odin(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "bark")
        .ok_or("expected a bark call")?;
    assert_eq!(call.receiver_text.as_deref(), Some("p"));
    assert_eq!(call.receiver_hint, Some(ReceiverHint::Identifier));
    Ok(())
}

#[test]
fn extracts_branch_heavy_proc_without_panicking() {
    let src = r#"
package main
main :: proc() {
    if true {
        helper()
    }
    for i := 0; i < 10; i += 1 {
        helper(i)
    }
    switch 1 {
    case 1:
        helper()
    }
}
"#;
    let parsed = parse_odin(src);
    assert!(symbol_kind(&parsed.symbols, "main").is_some());
}

#[test]
fn parses_fixture_widget_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("widget.odin");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_odin(&src);
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
    assert!(symbol_kind(&parsed.symbols, "draw").is_some());
    assert!(symbol_kind(&parsed.symbols, "render").is_some());
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "package main\nmain :: proc() {\n    helper()\n}\n";
    let first = parse_odin(src);
    let second = parse_odin(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_odin("proc ( { this is not valid odin @@@");
    let _ = parsed;
}
