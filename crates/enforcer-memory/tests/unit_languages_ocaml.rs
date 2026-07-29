//! Hard tests for OCaml, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_ocaml`])
//! -- there is no bespoke `languages::ocaml` extractor to prove
//! zero-regression against (OCaml has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::ocaml`]'s own doc
//! comment directly: `value_definition` -> `let_binding` -> `pattern`
//! name recovery, `constructor_declaration`/`method_definition`'s own
//! unfielded-child naming (a real gap this row closes past the
//! baseline's own resolver), `class_definition`/`module_definition`'s
//! two-level `_binding` unwrap, `type_definition`'s one-level
//! `type_binding` unwrap, `open_module`'s `module`-field IMPORTS, and
//! curried `application_expression`/`infix_expression`/
//! `method_invocation`/`module_application`/`new_expression` callee
//! reconstruction.

use enforcer_domain::memory_types::ReceiverHint;
use enforcer_memory::languages::generic::parse_ocaml;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_ocaml";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_value_definition_as_function() {
    let src = "let helper x = x + 1\n";
    let parsed = parse_ocaml(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_type_definition_as_type_alias() {
    let src = "type shape = Circle of float | Rectangle of float * float\n";
    let parsed = parse_ocaml(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "shape"),
        Some(&SymbolKind::TypeAlias),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_data_constructors_as_functions() {
    let src = "type shape = Circle of float | Rectangle of float * float\n";
    let parsed = parse_ocaml(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Circle"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Rectangle"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_class_definition_as_class() {
    let src = r#"
class widget =
  object
    method draw = print_string "draw"
  end
"#;
    let parsed = parse_ocaml(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "widget"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_method_definition_as_method() {
    let src = r#"
class widget =
  object
    method draw = print_string "draw"
  end
"#;
    let parsed = parse_ocaml(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "draw"),
        Some(&SymbolKind::Method),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_module_definition_as_module() {
    let src = "module Helper = struct\n  let square x = x * x\nend\n";
    let parsed = parse_ocaml(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Helper"),
        Some(&SymbolKind::Module),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_nested_value_definition_inside_module() {
    let src = "module Helper = struct\n  let square x = x * x\nend\n";
    let parsed = parse_ocaml(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "square"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_exception_definition_as_function() {
    let src = "exception BadShape\n";
    let parsed = parse_ocaml(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "BadShape"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_open_module_import_path() {
    let src = "open Printf\n";
    let parsed = parse_ocaml(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"Printf"));
}

#[test]
fn ordinary_definition_is_not_misdetected_as_an_import() {
    let src = "let helper x = x + 1\n";
    let parsed = parse_ocaml(src);
    assert!(parsed.imports.is_empty(), "{:?}", parsed.imports);
}

#[test]
fn extracts_curried_application_callee() -> TestResult {
    let src = "let helper x = x + 1\nlet draw s = helper 3\n";
    let parsed = parse_ocaml(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.arg_texts, vec!["3".to_string()], "{call:?}");
    Ok(())
}

#[test]
fn extracts_multi_arg_curried_application_callee() -> TestResult {
    let src = "let combine a b = a + b\nlet draw = combine 1 2\n";
    let parsed = parse_ocaml(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "combine")
        .ok_or("expected a combine call")?;
    assert_eq!(
        call.arg_texts,
        vec!["1".to_string(), "2".to_string()],
        "{call:?}"
    );
    Ok(())
}

#[test]
fn extracts_infix_operator_as_callee() -> TestResult {
    let src = "let helper x = x + 1\n";
    let parsed = parse_ocaml(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "+")
        .ok_or("expected a + call")?;
    assert_eq!(
        call.arg_texts,
        vec!["x".to_string(), "1".to_string()],
        "{call:?}"
    );
    Ok(())
}

#[test]
fn extracts_method_invocation_callee_with_receiver() -> TestResult {
    let src = "let render w = w#draw\n";
    let parsed = parse_ocaml(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "draw")
        .ok_or("expected a draw call")?;
    assert_eq!(call.receiver_text.as_deref(), Some("w"), "{call:?}");
    assert_eq!(
        call.receiver_hint,
        Some(ReceiverHint::Identifier),
        "{call:?}"
    );
    Ok(())
}

#[test]
fn extracts_new_expression_callee() -> TestResult {
    let src = "let make () = new widget\n";
    let parsed = parse_ocaml(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.receiver_hint == Some(ReceiverHint::NewExpression))
        .ok_or("expected a new-expression call")?;
    assert_eq!(call.callee, "widget", "{call:?}");
    Ok(())
}

#[test]
fn call_inside_value_definition_records_from_symbol_scope() -> TestResult {
    let src = "let helper x = x + 1\nlet draw s = helper 3\n";
    let parsed = parse_ocaml(src);
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
    let parsed = parse_ocaml("let ??? = this is not valid ocaml @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.ml");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_ocaml(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "shape"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "helper"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "area"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.imports.iter().any(|i| i.module_path == "Printf"),
        "{:?}",
        parsed.imports
    );
    assert!(
        parsed.calls.iter().any(|c| c.callee == "helper"),
        "{:?}",
        parsed.calls
    );
    Ok(())
}
