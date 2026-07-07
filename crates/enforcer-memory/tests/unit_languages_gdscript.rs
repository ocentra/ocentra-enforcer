//! Hard tests for GDScript onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_gdscript`]
//! -- language-parity wave G2.1d). GDScript has no pre-existing bespoke
//! `languages::gdscript` extractor, so these tests assert directly
//! against the grammar's own real shape -- both the
//! `tree-sitter-gdscript` crate's own `node-types.json` and a real parse
//! tree dump (a scratch `cargo run` against a minimal crate depending on
//! `tree-sitter-gdscript` directly, which caught two wrong assumptions
//! `node-types.json` alone did not surface -- see `LangSpec::gdscript`'s
//! own doc comment for the specifics) -- not byte-for-byte parity with
//! prior behavior.

use enforcer_memory::languages::generic::parse_gdscript;
use enforcer_memory::parsers::{ReceiverHint, SymbolKind};
use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_class_and_function_symbols() {
    let src = r#"
class_name Widget

func draw() -> String:
	return "x"
"#;
    let parsed = parse_gdscript(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Widget"),
        Some(&SymbolKind::Class)
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "draw"),
        Some(&SymbolKind::Function)
    );
}

#[test]
fn extracts_nested_class_method_as_method() {
    let src = r#"
class Inner:
	func helper():
		pass
"#;
    let parsed = parse_gdscript(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Inner"),
        Some(&SymbolKind::Class)
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Method)
    );
    assert!(
        parsed
            .defines
            .iter()
            .any(|d| d.container_name == "Inner" && d.member_name == "helper"),
        "{:?}",
        parsed.defines
    );
}

#[test]
fn extracts_enum_symbol() {
    let src = "enum Status { IDLE, RUNNING }";
    let parsed = parse_gdscript(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Status"),
        Some(&SymbolKind::Enum)
    );
}

#[test]
fn extracts_variable_export_onready_and_signal_fields_as_defines() {
    // Wrapped in a real, body-bearing `class Widget: ...` (the nested
    // inner-class syntax, `class_definition`) rather than the top-level
    // `class_name Widget` statement: `class_name_statement` has no
    // `body` field at all (it is a bodyless, whole-file annotation, see
    // `LangSpec::gdscript`'s own doc comment), so a file's ordinary
    // top-level fields have no syntactic container to DEFINES-link to --
    // this test instead exercises the DEFINES mechanism the same way
    // `extracts_nested_class_method_as_method` above already does, for
    // fields rather than a method.
    let src = r#"
class Widget:
	signal drawn(label)

	@export var label: String = "widget"
	onready var helper = get_node("Helper")

	var count: int = 0
"#;
    let parsed = parse_gdscript(src);
    let members: Vec<&str> = parsed
        .defines
        .iter()
        .filter(|d| d.container_name == "Widget")
        .map(|d| d.member_name.as_str())
        .collect();
    assert!(members.contains(&"drawn"), "{members:?}");
    assert!(members.contains(&"label"), "{members:?}");
    assert!(members.contains(&"helper"), "{members:?}");
    assert!(members.contains(&"count"), "{members:?}");
}

#[test]
fn extracts_extends_statement_as_import() {
    let src = r#"extends "res://base_widget.gd"
class_name Widget
"#;
    let parsed = parse_gdscript(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"res://base_widget.gd"), "{paths:?}");
}

#[test]
fn extracts_class_name_statement_extends_clause_as_import() {
    let src = "class_name Widget extends Node\n";
    let parsed = parse_gdscript(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"Node"), "{paths:?}");
}

#[test]
fn extracts_annotation_as_decorates_on_following_declaration() {
    let src = r#"
class_name Widget

@export var label: String = "widget"
"#;
    let parsed = parse_gdscript(src);
    assert!(
        parsed
            .decorates
            .iter()
            .any(|d| d.target_name == "label" && d.decorator_name == "export"),
        "{:?}",
        parsed.decorates
    );
}

#[test]
fn extracts_bare_call_edge() {
    let src = r#"
func f():
	helper()
"#;
    let parsed = parse_gdscript(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"helper"), "{callees:?}");
}

#[test]
fn extracts_attribute_call_edge_with_identifier_receiver() -> TestResult {
    let src = r#"
func f():
	helper.register()
"#;
    let parsed = parse_gdscript(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper.register")
        .ok_or("expected a helper.register call")?;
    assert_eq!(call.receiver_text.as_deref(), Some("helper"));
    assert_eq!(call.receiver_hint, Some(ReceiverHint::Identifier));
    Ok(())
}

#[test]
fn extracts_self_attribute_call_with_self_or_this_hint() -> TestResult {
    let src = r#"
func f():
	self.draw()
"#;
    let parsed = parse_gdscript(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "self.draw")
        .ok_or("expected a self.draw call")?;
    assert_eq!(call.receiver_hint, Some(ReceiverHint::SelfOrThis));
    Ok(())
}

#[test]
fn extracts_super_dot_call_as_ordinary_attribute_call() -> TestResult {
    // `super.draw()` is NOT `base_call` -- it parses as an ordinary
    // `attribute`/`attribute_call` pair with `super` as a perfectly
    // ordinary identifier receiver (confirmed via a real parse tree
    // dump; see `LangSpec::gdscript`'s own doc comment).
    let src = r#"
func draw():
	super.draw()
"#;
    let parsed = parse_gdscript(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "super.draw")
        .ok_or("expected a super.draw call")?;
    assert_eq!(call.receiver_text.as_deref(), Some("super"));
    assert_eq!(call.receiver_hint, Some(ReceiverHint::SelfOrThis));
    Ok(())
}

#[test]
fn extracts_base_call_dot_prefix_syntax() -> TestResult {
    // GDScript's actual `base_call` grammar rule: a bare leading-dot
    // call with NO receiver written at all (`.draw()`, "call the base
    // class's implementation" idiom) -- distinct from `super.draw()`
    // above. Confirmed via a real parse tree dump against this
    // grammar's `base_call: ($) => seq(".", $.identifier,
    // field("arguments", $.arguments))` rule.
    let src = "func draw():\n\t.draw()\n";
    let parsed = parse_gdscript(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "draw")
        .ok_or("expected a base_call recorded as callee \"draw\"")?;
    // No receiver text at all was written in source -- see
    // `gdscript_receiver_of_call`'s own `base_call` doc comment for why
    // this is `None`/`None` rather than an invented receiver.
    assert_eq!(call.receiver_text, None);
    assert_eq!(call.receiver_hint, None);
    Ok(())
}

#[test]
fn call_inside_function_records_from_symbol_scope() -> TestResult {
    let src = r#"
func render():
	helper()
"#;
    let parsed = parse_gdscript(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("render"));
    Ok(())
}

#[test]
fn extracts_call_args() -> TestResult {
    let src = r#"
func f():
	helper(1, "x")
"#;
    let parsed = parse_gdscript(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(
        call.arg_texts,
        vec!["1".to_string(), "\"x\"".to_string()],
        "{call:?}"
    );
    Ok(())
}

#[test]
fn extracts_branch_heavy_function_without_panicking() {
    let src = r#"
func increment(amount):
	var total = 0
	if amount > 0:
		total += amount
	else:
		total += 1
	for i in range(amount):
		total += i
	while total > 1000:
		total -= 1
	match amount:
		0:
			pass
		_:
			pass
	return total
"#;
    let parsed = parse_gdscript(src);
    assert!(symbol_kind(&parsed.symbols, "increment").is_some());
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = r#"
extends Node
class_name Widget

signal drawn(label)

func draw() -> String:
	return "x"
"#;
    let first = parse_gdscript(src);
    let second = parse_gdscript(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_gdscript("func ( { this is not valid gdscript @@@");
    let _ = parsed;
}
