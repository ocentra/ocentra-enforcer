//! Hard tests for Ada, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_ada`])
//! -- there is no bespoke `languages::ada` extractor to prove
//! zero-regression against (Ada has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::ada`]'s own doc comment
//! directly: subprogram naming off a nested specification node,
//! `full_type_declaration`'s positional (fieldless) name plus its
//! `derived_type_definition` INHERITS, `with`/`use` clause IMPORTS, and
//! `function_call`/`procedure_call_statement` callee reconstruction off
//! their own real `name` field.

use enforcer_memory::languages::generic::parse_ada;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_ada";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_package_and_subprogram_symbols() {
    let src = r#"
package body Widget is
   procedure Draw is
   begin
      null;
   end Draw;
end Widget;
"#;
    let parsed = parse_ada(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Widget"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Draw"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_function_symbol_via_nested_specification_name() -> TestResult {
    let src = r#"
package body Widget is
   function Helper (Label : String) return String is
   begin
      return Label;
   end Helper;
end Widget;
"#;
    let parsed = parse_ada(src);
    let sym = parsed
        .symbols
        .iter()
        .find(|s| s.name == "Helper")
        .ok_or("expected a Helper symbol")?;
    assert_eq!(sym.kind, SymbolKind::Function, "{sym:?}");
    Ok(())
}

#[test]
fn subprogram_defines_to_enclosing_package() -> TestResult {
    let src = r#"
package body Widget is
   procedure Draw is
   begin
      null;
   end Draw;
end Widget;
"#;
    let parsed = parse_ada(src);
    let defines: Vec<(&str, &str)> = parsed
        .defines
        .iter()
        .map(|d| (d.container_name.as_str(), d.member_name.as_str()))
        .collect();
    assert!(defines.contains(&("Widget", "Draw")));
    Ok(())
}

#[test]
fn extracts_full_type_declaration_symbol_via_positional_name() -> TestResult {
    let src = r#"
package Widget is
   type Widget_Type is record
      Name : String (1 .. 10);
   end record;
end Widget;
"#;
    let parsed = parse_ada(src);
    let sym = parsed
        .symbols
        .iter()
        .find(|s| s.name == "Widget_Type")
        .ok_or("expected a Widget_Type symbol")?;
    assert_eq!(sym.kind, SymbolKind::Class, "{sym:?}");
    Ok(())
}

#[test]
fn extracts_record_component_defines() -> TestResult {
    let src = r#"
package Widget is
   type Widget_Type is record
      Name : String (1 .. 10);
   end record;
end Widget;
"#;
    let parsed = parse_ada(src);
    let defines: Vec<(&str, &str)> = parsed
        .defines
        .iter()
        .map(|d| (d.container_name.as_str(), d.member_name.as_str()))
        .collect();
    assert!(defines.contains(&("Widget_Type", "Name")));
    Ok(())
}

#[test]
fn extracts_derived_type_as_inherits() -> TestResult {
    let src = r#"
package Widget is
   type Animal_Type is record
      Name : String (1 .. 10);
   end record;

   type Dog_Type is new Animal_Type;
end Widget;
"#;
    let parsed = parse_ada(src);
    let inherits: Vec<(&str, &str)> = parsed
        .inherits
        .iter()
        .map(|i| (i.sub_name.as_str(), i.super_name.as_str()))
        .collect();
    assert!(
        inherits.contains(&("Dog_Type", "Animal_Type")),
        "{inherits:?}"
    );
    Ok(())
}

#[test]
fn plain_type_declaration_has_no_inherits() {
    let src = r#"
package Widget is
   type Widget_Type is record
      Name : String (1 .. 10);
   end record;
end Widget;
"#;
    let parsed = parse_ada(src);
    assert!(parsed.inherits.is_empty(), "{:?}", parsed.inherits);
}

#[test]
fn extracts_procedure_call_statement() -> TestResult {
    let src = r#"
package body Widget is
   procedure Draw is
   begin
      Helper ("x");
   end Draw;
end Widget;
"#;
    let parsed = parse_ada(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "Helper")
        .ok_or("expected a Helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("Draw"), "{call:?}");
    Ok(())
}

#[test]
fn extracts_with_clause_import() {
    let src = r#"
with Ada.Text_IO;

package Widget is
end Widget;
"#;
    let parsed = parse_ada(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"Ada.Text_IO"));
}

#[test]
fn extracts_use_clause_import() {
    let src = r#"
use Ada.Text_IO;

package Widget is
end Widget;
"#;
    let parsed = parse_ada(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"Ada.Text_IO"));
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_ada("package ( { this is not valid ada @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.adb");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_ada(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Widget"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Helper"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Draw"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed
            .imports
            .iter()
            .any(|i| i.module_path == "Ada.Text_IO"),
        "{:?}",
        parsed.imports
    );
    assert!(
        parsed
            .inherits
            .iter()
            .any(|i| i.sub_name == "Dog_Type" && i.super_name == "Animal_Type"),
        "{:?}",
        parsed.inherits
    );
    Ok(())
}
