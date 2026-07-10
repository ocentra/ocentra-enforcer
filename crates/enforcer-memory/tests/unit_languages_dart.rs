//! Hard tests for Dart, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_dart`])
//! -- there is no bespoke `languages::dart` extractor to prove
//! zero-regression against (Dart has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::dart`]'s own doc
//! comment directly: symbol kinds (function/method/class/enum/
//! type-alias), `extends`/`implements` heritage (INHERITS/IMPLEMENTS),
//! call edges, and import/export URI extraction.

use enforcer_memory::languages::generic::parse_dart;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_dart";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_function_class_and_method_symbols() {
    let src = r#"
class Widget {
  String draw() {
    return "x";
  }
}

void helper() {}
"#;
    let parsed = parse_dart(src);
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
    assert_eq!(
        symbol_kind(&parsed.symbols, "draw"),
        Some(&SymbolKind::Method),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn method_defines_edge_targets_enclosing_class() {
    let src = r#"
class Widget {
  String draw() {
    return "x";
  }
}
"#;
    let parsed = parse_dart(src);
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
enum Status { active, inactive }

typedef IntCallback = void Function(int x);
"#;
    let parsed = parse_dart(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Status"),
        Some(&SymbolKind::Enum),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "IntCallback"),
        Some(&SymbolKind::TypeAlias),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_extends_as_inherits_and_implements_as_implements() {
    let src = r#"
class Base {}

abstract class Drawable {
  String draw();
}

abstract class Sizeable {
  int size();
}

class Widget extends Base implements Drawable, Sizeable {
  String draw() { return "x"; }
  int size() { return 0; }
}
"#;
    let parsed = parse_dart(src);
    let inherits: Vec<(&str, &str)> = parsed
        .inherits
        .iter()
        .map(|i| (i.sub_name.as_str(), i.super_name.as_str()))
        .collect();
    assert!(inherits.contains(&("Widget", "Base")), "{inherits:?}");

    let implements: Vec<(&str, &str)> = parsed
        .implements
        .iter()
        .map(|i| (i.type_name.as_str(), i.trait_name.as_str()))
        .collect();
    assert!(
        implements.contains(&("Widget", "Drawable")),
        "{implements:?}"
    );
    assert!(
        implements.contains(&("Widget", "Sizeable")),
        "{implements:?}"
    );
}

#[test]
fn extracts_call_edges_with_receiver() -> TestResult {
    let src = r#"
void f() {
  helper();
  w.draw();
}
"#;
    let parsed = parse_dart(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"helper"), "{callees:?}");
    let method_call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "w.draw")
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
void render() {
  helper();
}
"#;
    let parsed = parse_dart(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("render"), "{call:?}");
    Ok(())
}

#[test]
fn extracts_import_and_export_uris() {
    let src = r#"
import 'dart:async';
import 'package:foo/bar.dart' as bar;
export 'src/widget_base.dart';
"#;
    let parsed = parse_dart(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"dart:async"), "{paths:?}");
    assert!(paths.contains(&"package:foo/bar.dart"), "{paths:?}");
    assert!(paths.contains(&"src/widget_base.dart"), "{paths:?}");
}

#[test]
fn branch_nodes_are_recognized_by_the_shared_walk() {
    // The generic walker does not compute complexity itself (that is
    // `crate::complexity`'s job over the same `branch_types` array via
    // `NodeKindTable`) -- this asserts the fixture's branches parse
    // without panicking and the enclosing function/method is still
    // extracted correctly around them (a stand-in proxy for "branch
    // node kinds are valid and do not break the walk").
    let src = r#"
class Widget {
  String draw(String name) {
    if (name.isEmpty) {
      return "x";
    }
    for (var i = 0; i < 3; i++) {
      helper();
    }
    while (name.isEmpty) {
      break;
    }
    switch (name) {
      case "a":
        break;
      default:
        break;
    }
    return name;
  }
}

void helper() {}
"#;
    let parsed = parse_dart(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "draw"),
        Some(&SymbolKind::Method),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_dart("class ( { this is not valid dart @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.dart");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_dart(&src);
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
        parsed.imports.iter().any(|i| i.module_path == "dart:async"),
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
            .implements
            .iter()
            .any(|i| i.type_name == "Widget" && i.trait_name == "Drawable"),
        "{:?}",
        parsed.implements
    );
    Ok(())
}

#[test]
fn annotated_class_and_method_record_decorates_edges() -> TestResult {
    // language-parity wave G3 stage 3: Dart's `annotation` node has a
    // real `"name"` field, and is a direct, unfielded, positional child
    // of the class/method declaration it decorates (no `modifiers`
    // wrapper).
    let src = r#"
@immutable
class Widget {
    @override
    void draw() {}
}
"#;
    let parsed = parse_dart(src);
    let class_edge = parsed
        .decorates
        .iter()
        .find(|d| d.target_name == "Widget")
        .ok_or("expected a DECORATES edge for Widget")?;
    assert_eq!(class_edge.decorator_name, "immutable");
    let method_edge = parsed
        .decorates
        .iter()
        .find(|d| d.target_name == "draw")
        .ok_or("expected a DECORATES edge for draw")?;
    assert_eq!(method_edge.decorator_name, "override");
    Ok(())
}
