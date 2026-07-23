//! Hard tests for Groovy, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_groovy`])
//! -- there is no bespoke `languages::groovy` extractor to prove
//! zero-regression against (Groovy has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::groovy`]'s own doc
//! comment directly: symbol kinds (function/method/class), dotted
//! package Module symbol, `extends`/`implements` heritage (INHERITS/
//! IMPLEMENTS), receiver-qualified call edges (both `method_invocation`
//! and the parenthesis-less `juxt_function_call` idiom), and dotted
//! import paths.

use enforcer_memory::languages::generic::parse_groovy;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_groovy";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_package_as_module_symbol() {
    let parsed = parse_groovy("package com.example.widget\n");
    let names_kinds: Vec<(&str, SymbolKind)> = parsed
        .symbols
        .iter()
        .map(|s| (s.name.as_str(), s.kind))
        .collect();
    assert!(
        names_kinds.contains(&("com.example.widget", SymbolKind::Module)),
        "{names_kinds:?}"
    );
}

#[test]
fn extracts_function_class_and_method_symbols() {
    let src = r#"
class Widget {
    String draw() {
        return "x"
    }
}

def helper() {
    println "helping"
}
"#;
    let parsed = parse_groovy(src);
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
        return "x"
    }
}
"#;
    let parsed = parse_groovy(src);
    let defines: Vec<(&str, &str)> = parsed
        .defines
        .iter()
        .map(|d| (d.container_name.as_str(), d.member_name.as_str()))
        .collect();
    assert!(defines.contains(&("Widget", "draw")));
}

#[test]
fn extracts_extends_as_inherits_and_implements_as_implements() {
    let src = r#"
class Base {
}

interface Drawable {
    String draw()
}

class Widget extends Base implements Drawable {
    String draw() {
        return "x"
    }
}
"#;
    let parsed = parse_groovy(src);
    let inherits: Vec<(&str, &str)> = parsed
        .inherits
        .iter()
        .map(|i| (i.sub_name.as_str(), i.super_name.as_str()))
        .collect();
    assert!(inherits.contains(&("Widget", "Base")));

    let implements: Vec<(&str, &str)> = parsed
        .implements
        .iter()
        .map(|i| (i.type_name.as_str(), i.trait_name.as_str()))
        .collect();
    assert!(
        implements.contains(&("Widget", "Drawable")),
        "{implements:?}"
    );
}

#[test]
fn extracts_method_invocation_call_with_receiver() -> TestResult {
    let src = r#"
def f() {
    helper()
    w.draw()
}
"#;
    let parsed = parse_groovy(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"helper"));
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
fn extracts_juxt_function_call() -> TestResult {
    let src = r#"
def f() {
    println "hello"
}
"#;
    let parsed = parse_groovy(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "println")
        .ok_or("expected a println juxt call")?;
    let _ = call;
    Ok(())
}

#[test]
fn call_inside_function_records_from_symbol_scope() -> TestResult {
    let src = r#"
def render() {
    helper()
}
"#;
    let parsed = parse_groovy(src);
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
package com.example.widget

import com.example.other.Foo
import com.example.other.Bar
"#;
    let parsed = parse_groovy(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"com.example.other.Foo"));
    assert!(paths.contains(&"com.example.other.Bar"));
}

#[test]
fn branch_nodes_are_recognized_by_the_shared_walk() {
    let src = r#"
class Widget {
    String draw(String name) {
        if (name.isEmpty()) {
            return "x"
        }
        for (int i = 0; i < 3; i++) {
            helper()
        }
        while (name.isEmpty()) {
            break
        }
        return name
    }
}

def helper() {
}
"#;
    let parsed = parse_groovy(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "draw"),
        Some(&SymbolKind::Method),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_groovy("class ( { this is not valid groovy @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.groovy");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_groovy(&src);
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
            .any(|i| i.module_path == "com.example.other.Foo"),
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
    // language-parity wave G3 stage 3: Groovy's `marker_annotation`/
    // `annotation` nodes both have a real `"name"` field (identical
    // shape to Java's own annotation nodes) and sit inside an unfielded
    // `modifiers` wrapper child of the class/method declaration they
    // decorate.
    let src = r#"
@Deprecated
class Widget {
    @Override
    def draw() {}
}
"#;
    let parsed = parse_groovy(src);
    let class_edge = parsed
        .decorates
        .iter()
        .find(|d| d.target_name == "Widget")
        .ok_or("expected a DECORATES edge for Widget")?;
    assert_eq!(class_edge.decorator_name, "Deprecated");
    let method_edge = parsed
        .decorates
        .iter()
        .find(|d| d.target_name == "draw")
        .ok_or("expected a DECORATES edge for draw")?;
    assert_eq!(method_edge.decorator_name, "Override");
    Ok(())
}
