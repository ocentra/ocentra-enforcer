//! Hard tests for QML onboarded directly through the generic
//! spec-table engine ([`enforcer_syntax::languages::generic::parse_qml`]
//! -- language-parity wave G2.2e). QML has no pre-existing bespoke
//! `languages::qml` extractor, so these tests assert directly against
//! the grammar's own real shape -- both the `tree-sitter-qmljs` crate's
//! own `node-types.json` and a real parse tree dump (a scratch `cargo
//! run` against a minimal crate depending on that grammar's vendored
//! `parser.c` directly, which caught that a bare top-level statement
//! with no wrapping QML object at all does not parse cleanly -- see
//! `LangSpec::qml`'s own doc comment for the specifics) -- not
//! byte-for-byte parity with prior behavior. Every sample below wraps
//! its JS-shaped content inside a real top-level `Item { ... }`
//! object, matching that finding.

use enforcer_syntax::languages::generic::parse_qml;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_function_symbol_alongside_top_level_ui_property_and_signal() {
    let src = r#"
import QtQuick

Item {
    property int count: 0
    signal clicked(int x)

    function increment() {
        count = count + 1;
    }
}
"#;
    let parsed = parse_qml(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "increment"),
        Some(&SymbolKind::Function)
    );
}

#[test]
fn ui_property_defines_into_enclosing_ui_inline_component_when_nested() {
    // A top-level QML object (`Item {...}`) is never itself a
    // recognized class-shaped node (matches the baseline's own
    // `qml_class_types` never listing `ui_object_definition` either),
    // so a `ui_property`/`ui_signal` directly inside one has no
    // `enclosing` container to DEFINE into at all -- but nested inside
    // a `ui_inline_component` (which DOES set `enclosing` to its own
    // name via `qml_walk_scoped_body`), the identical field correctly
    // produces a DEFINES edge.
    let src = r#"
import QtQuick

Item {
    component Circle: Rectangle {
        property int radius: 50
    }
}
"#;
    let parsed = parse_qml(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Circle"),
        Some(&SymbolKind::Class)
    );
    assert!(
        parsed
            .defines
            .iter()
            .any(|d| d.container_name == "Circle" && d.member_name == "radius"),
        "{:?}",
        parsed.defines
    );
}

#[test]
fn extracts_ui_inline_component_as_class_symbol() {
    let src = r#"
import QtQuick

Item {
    component Circle: Rectangle {
        radius: 50
    }
}
"#;
    let parsed = parse_qml(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Circle"),
        Some(&SymbolKind::Class)
    );
}

#[test]
fn extracts_class_declaration_and_method_nested_in_function() {
    let src = r#"
import QtQuick

Item {
    function makeWidget() {
        class Widget {
            draw() {
                return 1;
            }
        }
        let w = new Widget();
        w.draw();
        return w;
    }
}
"#;
    let parsed = parse_qml(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Widget"),
        Some(&SymbolKind::Class)
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "draw"),
        Some(&SymbolKind::Method)
    );
}

#[test]
fn extracts_import_statements_including_ui_import_alias_form() {
    let src = r#"
import QtQuick
import "./helpers.js" as Helpers

Item {
}
"#;
    let parsed = parse_qml(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"QtQuick"));
    assert!(paths.contains(&"./helpers.js"));
}

#[test]
fn quoted_qml_import_path_is_unquoted() {
    let parsed = parse_qml(
        r#"
import "./quoted-helper.js"

Item {}
"#,
    );

    assert!(
        parsed
            .imports
            .iter()
            .any(|import| import.module_path == "./quoted-helper.js"),
        "{:?}",
        parsed.imports
    );
}

#[test]
fn extracts_call_edges_including_new_expression() -> TestResult {
    let src = r#"
import QtQuick

Item {
    function makeWidget() {
        class Widget {
            draw() {
                return 1;
            }
        }
        let w = new Widget();
        w.draw();
    }
}
"#;
    let parsed = parse_qml(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"Widget"));
    assert!(callees.contains(&"w.draw"));
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "w.draw")
        .ok_or("expected a w.draw call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("makeWidget"));
    Ok(())
}

#[test]
fn call_inside_function_records_from_symbol_scope() -> TestResult {
    let src = r#"
import QtQuick

Item {
    function f() {
        helper();
    }
}
"#;
    let parsed = parse_qml(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("f"));
    Ok(())
}

#[test]
fn extracts_branch_heavy_function_without_panicking() {
    let src = r#"
import QtQuick

Item {
    function f(x) {
        if (x > 0) {
            return 1;
        } else {
            return 2;
        }
        for (let i = 0; i < 10; i++) {
            console.log(i);
        }
        while (x < 5) {
            x++;
        }
        switch (x) {
            case 1:
                break;
            default:
                break;
        }
        try {
            risky();
        } catch (e) {
            console.log(e);
        }
    }
}
"#;
    let parsed = parse_qml(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "f"),
        Some(&SymbolKind::Function)
    );
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = r#"
import QtQuick

Item {
    property int count: 0

    function increment() {
        count = count + 1;
    }
}
"#;
    let first = parse_qml(src);
    let second = parse_qml(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_qml("Item { this is not valid qml @@@");
    let _ = parsed;
}

#[test]
fn real_fixture_file_parses_and_extracts_expected_symbols() {
    let src = include_str!("fixtures/memory/lang_qml/Widget.qml");
    let parsed = parse_qml(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "increment"),
        Some(&SymbolKind::Function)
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Circle"),
        Some(&SymbolKind::Class)
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Helper"),
        Some(&SymbolKind::Class)
    );
    assert!(
        parsed
            .defines
            .iter()
            .any(|d| d.container_name == "Circle" && d.member_name == "radius"),
        "{:?}",
        parsed.defines
    );
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"QtQuick"));
}
