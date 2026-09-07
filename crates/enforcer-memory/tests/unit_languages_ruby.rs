//! Hard tests for Ruby, onboarded directly through the generic
//! spec-table engine ([`enforcer_syntax::languages::generic::parse_ruby`])
//! -- there is no bespoke `languages::ruby` extractor to prove
//! zero-regression against (Ruby has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::ruby`]'s own doc
//! comment directly: symbol kinds (method/singleton_method/class/
//! module), `superclass`-field INHERITS, `receiver.method` call
//! reconstruction (incl. the baseline's `Widget.new(...)` constructor
//! redirect), `require`/`require_relative` IMPORTS, and DEFINES-scoped
//! class bodies.

use enforcer_domain::memory_types::ReceiverHint;
use enforcer_syntax::languages::generic::parse_ruby;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_ruby";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_class_module_and_method_symbols() {
    let src = r#"
module Shapes
  class Widget
    def draw
      "widget"
    end
  end
end
"#;
    let parsed = parse_ruby(src);
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
    let parsed = parse_ruby(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_singleton_method_as_method() {
    let src = r#"
class Widget
  def self.create
    Widget.new
  end
end
"#;
    let parsed = parse_ruby(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "create"),
        Some(&SymbolKind::Method),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_superclass_as_inherits() {
    let src = r#"
class Animal
end

class Dog < Animal
end
"#;
    let parsed = parse_ruby(src);
    let inherits: Vec<(&str, &str)> = parsed
        .inherits
        .iter()
        .map(|i| (i.sub_name.as_str(), i.super_name.as_str()))
        .collect();
    assert!(inherits.contains(&("Dog", "Animal")));
}

#[test]
fn class_with_no_superclass_has_no_inherits() {
    let src = r#"
class Standalone
end
"#;
    let parsed = parse_ruby(src);
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
    let parsed = parse_ruby(src);
    let defines: Vec<(&str, &str)> = parsed
        .defines
        .iter()
        .map(|d| (d.container_name.as_str(), d.member_name.as_str()))
        .collect();
    assert!(defines.contains(&("Widget", "draw")));
    assert!(defines.contains(&("Widget", "resize")));
}

#[test]
fn extracts_receiver_qualified_call() -> TestResult {
    let src = r#"
def render(w)
  w.draw
end
"#;
    let parsed = parse_ruby(src);
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
    // NOTE: explicit parens (`helper()`), not the bare `helper` form --
    // a truly bare, argument-less, receiver-less identifier reference is
    // NOT a `call`-kind node at all in this grammar (verified against a
    // real parse: it is indistinguishable from a local-variable
    // reference and surfaces as a plain `identifier` node), matching
    // the baseline's own inability to classify that shape as a call
    // either (`ruby_call_types` only ever matches `call`/`command_call`,
    // neither of which a bare identifier ever is).
    let src = r#"
def f
  helper()
end
"#;
    let parsed = parse_ruby(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.receiver_text, None, "{call:?}");
    assert_eq!(call.receiver_hint, None, "{call:?}");
    Ok(())
}

#[test]
fn call_with_arguments_records_arg_texts() -> TestResult {
    let src = r#"
def f(w)
  w.resize(10, 20)
end
"#;
    let parsed = parse_ruby(src);
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
fn constructor_call_redirects_callee_to_receiver_type_name() -> TestResult {
    // Baseline `internal/cbm/extract_calls.c` CBM_LANG_RUBY redirect:
    // `Widget.new(...)` records the callee as `"Widget"`, not `"new"`
    // (Ruby's constructor body lives in `initialize`, so a bare `"new"`
    // callee would never resolve to anything).
    let src = r#"
def build
  Widget.new
end
"#;
    let parsed = parse_ruby(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "Widget")
        .ok_or("expected callee redirected to Widget")?;
    assert_eq!(
        call.receiver_hint,
        Some(ReceiverHint::NewExpression),
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
    let parsed = parse_ruby(src);
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
fn extracts_require_and_require_relative_as_imports() {
    let src = r#"
require 'json'
require_relative './helper'
"#;
    let parsed = parse_ruby(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"json"));
    assert!(paths.contains(&"./helper"));
}

#[test]
fn require_call_is_also_recorded_as_a_call() {
    let src = "require 'json'\n";
    let parsed = parse_ruby(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"require"));
}

#[test]
fn ordinary_call_is_not_misdetected_as_an_import() {
    let src = r#"
def f
  helper("x")
end
"#;
    let parsed = parse_ruby(src);
    assert!(parsed.imports.is_empty(), "{:?}", parsed.imports);
}

#[test]
fn call_inside_method_records_from_symbol_scope() -> TestResult {
    // Explicit parens -- see `unqualified_call_has_no_receiver`'s own
    // note on why a bare `helper` (no parens) never parses as a `call`
    // node at all in this grammar.
    let src = r#"
class Widget
  def render
    helper()
  end
end
"#;
    let parsed = parse_ruby(src);
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
    let parsed = parse_ruby(src);
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
    let parsed = parse_ruby("class ( { this is not valid ruby @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.rb");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_ruby(&src);
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
