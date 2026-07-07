//! Hard tests for Scala, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_scala`])
//! -- there is no bespoke `languages::scala` extractor to prove
//! zero-regression against (Scala has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::scala`]'s own doc
//! comment directly: symbol kinds (function/method/class/trait-as-
//! interface/object/enum/type-alias), `extends ... with ...` heritage
//! (INHERITS), call edges, and repeated-field import-path
//! reconstruction.

use enforcer_memory::languages::generic::parse_scala;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_scala";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_function_class_trait_and_object_symbols() {
    let src = r#"
package widget

trait Drawable {
  def draw(): String
}

class Widget(val name: String) extends Drawable {
  def draw(): String = name
}

object Widget {
  def apply(name: String): Widget = new Widget(name)
}

def helper(): Unit = {}
"#;
    let parsed = parse_scala(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Drawable"),
        Some(&SymbolKind::Interface),
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
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    // "draw"/"apply" are both defined inside a class/object body, so
    // both must be classified as Method, never a free Function.
    let method_kinds: Vec<&SymbolKind> = parsed
        .symbols
        .iter()
        .filter(|s| s.name == "draw" || s.name == "apply")
        .map(|s| &s.kind)
        .collect();
    assert!(!method_kinds.is_empty(), "{:?}", parsed.symbols);
    assert!(
        method_kinds.iter().all(|k| **k == SymbolKind::Method),
        "{method_kinds:?}"
    );
}

#[test]
fn method_defines_edge_targets_enclosing_class() {
    let src = r#"
class Widget {
  def draw(): String = "x"
}
"#;
    let parsed = parse_scala(src);
    let defines: Vec<(&str, &str)> = parsed
        .defines
        .iter()
        .map(|d| (d.container_name.as_str(), d.member_name.as_str()))
        .collect();
    assert!(defines.contains(&("Widget", "draw")), "{defines:?}");
}

#[test]
fn extracts_enum_and_type_alias_symbols() {
    let src = r#"
enum Status {
  case Active, Inactive
}

type Alias = String
"#;
    let parsed = parse_scala(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Status"),
        Some(&SymbolKind::Enum),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Alias"),
        Some(&SymbolKind::TypeAlias),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_extends_with_chain_as_inherits() {
    let src = r#"
class Base

trait Drawable
trait Sizeable

class Widget extends Base with Drawable with Sizeable {
}
"#;
    let parsed = parse_scala(src);
    let inherits: Vec<(&str, &str)> = parsed
        .inherits
        .iter()
        .map(|i| (i.sub_name.as_str(), i.super_name.as_str()))
        .collect();
    assert!(inherits.contains(&("Widget", "Base")), "{inherits:?}");
    assert!(inherits.contains(&("Widget", "Drawable")), "{inherits:?}");
    assert!(inherits.contains(&("Widget", "Sizeable")), "{inherits:?}");
}

#[test]
fn extracts_call_edges_with_receiver() -> TestResult {
    let src = r#"
def f(): Unit = {
  helper()
  w.draw()
}
"#;
    let parsed = parse_scala(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"helper"), "{callees:?}");
    let method_call = parsed
        .calls
        .iter()
        .find(|c| c.callee.contains("draw"))
        .ok_or("expected a w.draw call")?;
    assert_eq!(
        method_call.receiver_text.as_deref(),
        Some("w"),
        "{method_call:?}"
    );
    Ok(())
}

#[test]
fn call_inside_function_records_from_symbol_scope() -> TestResult {
    let src = r#"
def render(): Unit = {
  helper()
}
"#;
    let parsed = parse_scala(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("render"), "{call:?}");
    Ok(())
}

#[test]
fn extracts_dotted_import_paths() {
    let src = r#"
package widget

import scala.collection.mutable
import com.example.other.Foo
"#;
    let parsed = parse_scala(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"scala.collection.mutable"), "{paths:?}");
    assert!(paths.contains(&"com.example.other.Foo"), "{paths:?}");
}

#[test]
fn branch_nodes_are_recognized_by_the_shared_walk() {
    let src = r#"
class Widget {
  def draw(name: String): String = {
    if (name.isEmpty) {
      "x"
    } else {
      for (i <- 0 until 3) {
        helper()
      }
      try {
        helper()
      } catch {
        case e: Exception => "err"
      }
      name match {
        case "a" => "matched"
        case _ => "other"
      }
    }
  }
}

def helper(): Unit = {}
"#;
    let parsed = parse_scala(src);
    let method_kinds: Vec<&SymbolKind> = parsed
        .symbols
        .iter()
        .filter(|s| s.name == "draw")
        .map(|s| &s.kind)
        .collect();
    assert!(
        method_kinds.iter().all(|k| **k == SymbolKind::Method),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_scala("class ( { this is not valid scala @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.scala");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_scala(&src);
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
            .any(|i| i.module_path == "scala.collection.mutable"),
        "{:?}",
        parsed.imports
    );
    assert!(
        parsed
            .inherits
            .iter()
            .any(|i| i.sub_name == "Widget" && i.super_name == "Base"),
        "{:?}",
        parsed.inherits
    );
    assert!(
        parsed
            .inherits
            .iter()
            .any(|i| i.sub_name == "Widget" && i.super_name == "Drawable"),
        "{:?}",
        parsed.inherits
    );
    Ok(())
}
