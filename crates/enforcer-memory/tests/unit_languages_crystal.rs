//! Hard tests for Crystal, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_memory::languages::generic::parse_crystal`]) -- there is
//! no bespoke `languages::crystal` extractor to prove zero-regression
//! against (Crystal has never had one in this crate), so these tests
//! assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::crystal`]'s own doc
//! comment directly: symbol kinds (method/class/struct/module/enum),
//! `superclass`-field INHERITS, `receiver.method` call reconstruction,
//! `instance_var`/`class_var` DEFINES (sigil-prefixed node text as the
//! name), `require` IMPORTS, and DEFINES-scoped class bodies.

use enforcer_memory::languages::generic::parse_crystal;
use enforcer_memory::parsers::{ReceiverHint, SymbolKind};
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_crystal";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_module_class_and_method_symbols() {
    let src = r#"
module Shapes
  class Widget
    def draw
      "widget"
    end
  end
end
"#;
    let parsed = parse_crystal(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Shapes"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Widget"),
        Some(&SymbolKind::Class),
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
fn top_level_def_is_a_function_not_a_method() {
    let src = r#"
def helper(label)
  label.upcase
end
"#;
    let parsed = parse_crystal(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_struct_and_enum_symbols() {
    let src = r#"
struct Point
  property x : Int32
end

enum Status
  Active
  Inactive
end
"#;
    let parsed = parse_crystal(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Point"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Status"),
        Some(&SymbolKind::Enum),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_superclass_as_inherits() -> TestResult {
    let src = r#"
class Animal
end

class Dog < Animal
end
"#;
    let parsed = parse_crystal(src);
    let inherit = parsed
        .inherits
        .iter()
        .find(|i| i.sub_name == "Dog")
        .ok_or("expected Dog to inherit from something")?;
    assert_eq!(inherit.super_name, "Animal", "{inherit:?}");
    Ok(())
}

#[test]
fn class_with_no_superclass_has_no_inherits() {
    let src = r#"
class Standalone
end
"#;
    let parsed = parse_crystal(src);
    assert!(parsed.inherits.is_empty(), "{:?}", parsed.inherits);
}

#[test]
fn extracts_method_defines_inside_class_body() {
    let src = r#"
class Widget
  def draw
  end

  def resize
  end
end
"#;
    let parsed = parse_crystal(src);
    let defines: Vec<(&str, &str)> = parsed
        .defines
        .iter()
        .map(|d| (d.container_name.as_str(), d.member_name.as_str()))
        .collect();
    assert!(defines.contains(&("Widget", "draw")), "{defines:?}");
    assert!(defines.contains(&("Widget", "resize")), "{defines:?}");
}

#[test]
fn extracts_instance_var_defines_with_sigil() {
    let src = r#"
class Widget
  @name : String
end
"#;
    let parsed = parse_crystal(src);
    let defines: Vec<(&str, &str)> = parsed
        .defines
        .iter()
        .map(|d| (d.container_name.as_str(), d.member_name.as_str()))
        .collect();
    assert!(defines.contains(&("Widget", "@name")), "{defines:?}");
}

#[test]
fn extracts_receiver_qualified_call() -> TestResult {
    let src = r#"
def render(w)
  w.draw
end
"#;
    let parsed = parse_crystal(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "w.draw")
        .ok_or("expected a w.draw call")?;
    assert_eq!(call.receiver_text.as_deref(), Some("w"), "{call:?}");
    assert_eq!(
        call.receiver_hint,
        Some(ReceiverHint::Identifier),
        "{call:?}"
    );
    Ok(())
}

#[test]
fn unqualified_call_has_no_receiver() -> TestResult {
    let src = r#"
def f
  helper()
end
"#;
    let parsed = parse_crystal(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.receiver_text, None, "{call:?}");
    Ok(())
}

#[test]
fn call_with_arguments_records_arg_texts() -> TestResult {
    let src = r#"
def f(w)
  w.resize(10, 20)
end
"#;
    let parsed = parse_crystal(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "w.resize")
        .ok_or("expected a w.resize call")?;
    assert_eq!(
        call.arg_texts,
        vec!["10".to_string(), "20".to_string()],
        "{call:?}"
    );
    Ok(())
}

#[test]
fn self_receiver_is_self_or_this_hint() -> TestResult {
    let src = r#"
class Widget
  def draw
    self.helper
  end
end
"#;
    let parsed = parse_crystal(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee.ends_with(".helper"))
        .ok_or("expected a self.helper call")?;
    assert_eq!(
        call.receiver_hint,
        Some(ReceiverHint::SelfOrThis),
        "{call:?}"
    );
    Ok(())
}

#[test]
fn extracts_require_as_import() {
    let src = "require \"json\"\n";
    let parsed = parse_crystal(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"json"), "{paths:?}");
}

#[test]
fn require_is_not_recorded_as_a_call() {
    let src = "require \"json\"\n";
    let parsed = parse_crystal(src);
    assert!(parsed.calls.is_empty(), "{:?}", parsed.calls);
}

#[test]
fn ordinary_call_is_not_misdetected_as_an_import() {
    let src = r#"
def f
  helper("x")
end
"#;
    let parsed = parse_crystal(src);
    assert!(parsed.imports.is_empty(), "{:?}", parsed.imports);
}

#[test]
fn call_inside_method_records_from_symbol_scope() -> TestResult {
    let src = r#"
class Widget
  def render
    helper()
  end
end
"#;
    let parsed = parse_crystal(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("render"), "{call:?}");
    Ok(())
}

#[test]
fn module_scope_call_has_no_from_symbol() -> TestResult {
    let src = "helper()\n";
    let parsed = parse_crystal(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol, None, "{call:?}");
    Ok(())
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_crystal("class ( { this is not valid crystal @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.cr");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_crystal(&src);
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
        parsed.imports.iter().any(|i| i.module_path == "json"),
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
