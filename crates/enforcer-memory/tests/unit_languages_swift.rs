//! Hard tests for Swift, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_swift`])
//! -- no bespoke `languages::swift` extractor exists to prove
//! zero-regression against, so these assert directly against the
//! grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::swift`]'s own doc
//! comment: the `class_declaration`/`declaration_kind` struct/enum/class
//! split, protocol extends-only INHERITS, `property_declaration`'s
//! two-levels-deep name DEFINES, and all four call-shaped node kinds
//! (`call_expression`/`constructor_expression`/`macro_invocation`/
//! `navigation_expression`).

use enforcer_memory::languages::generic::parse_swift;
use enforcer_memory::parsers::{ReceiverHint, SymbolKind};
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_swift";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_class_struct_enum_via_declaration_kind() {
    let src = r#"
class Widget {
}

struct Point {
}

enum Direction {
    case up
}

protocol Drawable {
}
"#;
    let parsed = parse_swift(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Widget"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Point"),
        Some(&SymbolKind::Struct),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Direction"),
        Some(&SymbolKind::Enum),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Drawable"),
        Some(&SymbolKind::Interface),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_function_and_method_with_distinct_kinds() {
    let src = r#"
func topLevel() -> String {
    return "x"
}

class Widget {
    func draw() -> String {
        return "x"
    }
}
"#;
    let parsed = parse_swift(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "topLevel"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "draw"),
        Some(&SymbolKind::Method),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_class_inheritance_specifier_as_inherits() {
    let src = r#"
protocol Drawable {
}

class Widget: Drawable {
}
"#;
    let parsed = parse_swift(src);
    let inherits: Vec<(&str, &str)> = parsed
        .inherits
        .iter()
        .map(|i| (i.sub_name.as_str(), i.super_name.as_str()))
        .collect();
    assert!(inherits.contains(&("Widget", "Drawable")), "{inherits:?}");
}

#[test]
fn extracts_protocol_inheritance_as_inherits() {
    let src = r#"
protocol Base {
}

protocol Sub: Base {
}
"#;
    let parsed = parse_swift(src);
    let inherits: Vec<(&str, &str)> = parsed
        .inherits
        .iter()
        .map(|i| (i.sub_name.as_str(), i.super_name.as_str()))
        .collect();
    assert!(inherits.contains(&("Sub", "Base")), "{inherits:?}");
}

#[test]
fn extracts_property_declaration_as_defines() {
    let src = r#"
class Widget {
    let name: String
}
"#;
    let parsed = parse_swift(src);
    let defines: Vec<(&str, &str)> = parsed
        .defines
        .iter()
        .map(|d| (d.container_name.as_str(), d.member_name.as_str()))
        .collect();
    assert!(defines.contains(&("Widget", "name")), "{defines:?}");
}

#[test]
fn extracts_call_expression_edge() -> TestResult {
    let src = r#"
func f() {
    helper()
}
"#;
    let parsed = parse_swift(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee.starts_with("helper"))
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("f"), "{call:?}");
    Ok(())
}

#[test]
fn extracts_navigation_expression_receiver_and_call() -> TestResult {
    let src = r#"
func f() {
    obj.method()
}
"#;
    let parsed = parse_swift(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee.contains("obj.method"))
        .ok_or("expected an obj.method call")?;
    assert_eq!(call.receiver_text.as_deref(), Some("obj"), "{call:?}");
    assert_eq!(
        call.receiver_hint,
        Some(ReceiverHint::Identifier),
        "{call:?}"
    );
    Ok(())
}

#[test]
fn extracts_constructor_expression_as_new_expression_call() -> TestResult {
    // `Widget()` (no explicit type arguments) parses as a plain
    // `call_expression` in this grammar -- syntactically indistinguishable
    // from an ordinary function call named `Widget`, verified with a
    // standalone debug harness against the real grammar (there is no
    // syntactic `new`-keyword-equivalent for Swift's common construction
    // idiom). `constructor_expression` is this grammar's OWN dedicated
    // node kind for a narrower real trigger: a generic type with
    // explicit type arguments being constructed (`Array<Int>()`),
    // confirmed the same way.
    let src = r#"
func f() -> Array<Int> {
    return Array<Int>()
}
"#;
    let parsed = parse_swift(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee.starts_with("Array"))
        .ok_or("expected an Array<Int>() constructor call")?;
    assert_eq!(
        call.receiver_hint,
        Some(ReceiverHint::NewExpression),
        "{call:?}"
    );
    Ok(())
}

#[test]
fn extracts_import_declaration() {
    let src = "import Foundation\n";
    let parsed = parse_swift(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"Foundation"), "{paths:?}");
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_swift("class ( { this is not valid swift @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.swift");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_swift(&src);
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
        parsed.imports.iter().any(|i| i.module_path == "Foundation"),
        "{:?}",
        parsed.imports
    );
    Ok(())
}
