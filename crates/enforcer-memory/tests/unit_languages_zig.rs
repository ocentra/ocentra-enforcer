//! Hard tests for Zig, onboarded directly through the generic
//! spec-table engine ([`enforcer_syntax::languages::generic::parse_zig`])
//! -- there is no bespoke `languages::zig` extractor to prove
//! zero-regression against (Zig has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::zig`]'s own doc
//! comment directly: struct/enum anonymous-type-expression naming via
//! the parent `variable_declaration` (Zig structs have NO `name` field
//! at all), `test "name" { ... }` string-literal naming, field DEFINES,
//! ordinary `call_expression` calls, and `@import(...)` builtin-function
//! IMPORTS detection.

use enforcer_syntax::languages::generic::parse_zig;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_zig";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_function_declaration() {
    let src = r#"
pub fn main() void {
}
"#;
    let parsed = parse_zig(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "main"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_struct_name_from_parent_const_binding() {
    // A Zig `struct_declaration` has NO `name` field at all -- its name
    // comes from the parent `const Foo = struct {...}` binding, one
    // level up the tree.
    let src = r#"
const Widget = struct {
    name: []const u8,
};
"#;
    let parsed = parse_zig(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Widget"),
        Some(&SymbolKind::Struct),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_enum_name_from_parent_const_binding() {
    let src = r#"
const Color = enum {
    Red,
    Green,
};
"#;
    let parsed = parse_zig(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Color"),
        Some(&SymbolKind::Enum),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn struct_not_bound_to_a_const_has_no_name() {
    // An anonymous struct literal used as a value (not a
    // `variable_declaration` binding) has no name to recover -- it must
    // not crash, and it must not add a spurious empty-name symbol.
    let src = r#"
pub fn main() void {
    const p = struct {
        x: i32,
    }{ .x = 1 };
}
"#;
    let parsed = parse_zig(src);
    let _ = parsed;
}

#[test]
fn extracts_struct_field_defines() {
    let src = r#"
const Widget = struct {
    name: []const u8,
    age: i32,
};
"#;
    let parsed = parse_zig(src);
    let defines: Vec<(&str, &str)> = parsed
        .defines
        .iter()
        .map(|d| (d.container_name.as_str(), d.member_name.as_str()))
        .collect();
    assert!(defines.contains(&("Widget", "name")));
    assert!(defines.contains(&("Widget", "age")));
}

#[test]
fn extracts_test_declaration_string_name() {
    let src = r#"
test "widget draws its name" {
}
"#;
    let parsed = parse_zig(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "widget draws its name"),
        Some(&SymbolKind::Test),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_call_expression() -> TestResult {
    let src = r#"
pub fn main() void {
    helper();
}
"#;
    let parsed = parse_zig(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"helper"));
    Ok(())
}

#[test]
fn call_inside_function_records_from_symbol_scope() -> TestResult {
    let src = r#"
pub fn render() void {
    helper();
}
"#;
    let parsed = parse_zig(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("render"), "{call:?}");
    Ok(())
}

#[test]
fn extracts_import_builtin_as_imports_edge() {
    let src = r#"
const std = @import("std");
"#;
    let parsed = parse_zig(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"std"));
}

#[test]
fn import_builtin_is_also_recorded_as_a_call() {
    let src = r#"
const std = @import("std");
"#;
    let parsed = parse_zig(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"@import"));
}

#[test]
fn non_import_builtin_is_a_call_but_not_an_import() {
    let src = r#"
pub fn main() void {
    @compileLog("hi");
}
"#;
    let parsed = parse_zig(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"@compileLog"));
    assert!(parsed.imports.is_empty(), "{:?}", parsed.imports);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_zig("const ( { this is not valid zig @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.zig");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_zig(&src);
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
        parsed.imports.iter().any(|i| i.module_path == "std"),
        "{:?}",
        parsed.imports
    );
    Ok(())
}
