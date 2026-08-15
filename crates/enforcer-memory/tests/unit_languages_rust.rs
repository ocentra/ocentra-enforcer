use enforcer_syntax::languages::rust::parse;
use enforcer_syntax::parsers::SymbolKind;

#[test]
fn extracts_function_and_test_symbols() {
    let src = r#"
fn normal_fn() {}

#[test]
fn a_test() {}

#[tokio::test]
async fn an_async_test() {}
"#;
    let parsed = parse(src);
    let names_kinds: Vec<(&str, SymbolKind)> = parsed
        .symbols
        .iter()
        .map(|s| (s.name.as_str(), s.kind))
        .collect();
    assert!(names_kinds.contains(&("normal_fn", SymbolKind::Function)));
    assert!(names_kinds.contains(&("a_test", SymbolKind::Test)));
    assert!(names_kinds.contains(&("an_async_test", SymbolKind::Test)));
}

#[test]
fn extracts_struct_enum_trait_with_distinct_kinds() {
    // X06 rich vocabulary: struct/enum/trait are now distinct labels
    // (Struct/Enum/Interface), not folded into one generic Type.
    let src = "struct Foo; enum Bar { A } trait Baz {}";
    let parsed = parse(src);
    let kinds: Vec<(&str, SymbolKind)> = parsed
        .symbols
        .iter()
        .map(|s| (s.name.as_str(), s.kind))
        .collect();
    assert!(kinds.contains(&("Foo", SymbolKind::Struct)));
    assert!(kinds.contains(&("Bar", SymbolKind::Enum)));
    assert!(kinds.contains(&("Baz", SymbolKind::Interface)));
}

#[test]
fn extracts_use_imports() {
    let src = "use crate::graph::MemoryGraph; use std::{fs, path::Path};";
    let parsed = parse(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"crate::graph::MemoryGraph"));
    assert!(paths.iter().any(|p| p.contains("fs")));
    assert!(paths.iter().any(|p| p.contains("Path")));
}

#[test]
fn extracts_call_edges() {
    let src = "fn f() { helper(); other::thing(1, 2); }";
    let parsed = parse(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"helper"));
    assert!(callees.contains(&"other::thing"));
}

/// Regression: iterator traversal retains source order through trait bounds,
/// grouped imports, and nested call arguments.
#[test]
fn rust_child_iteration_preserves_bounds_import_and_call_argument_order() -> Result<(), &'static str>
{
    let src = r#"
trait Child: First + Second {}
use crate::items::{Alpha, Beta};
fn run() { other::thing(first(), second()); }
"#;
    let parsed = parse(src);
    let bounds: Vec<&str> = parsed
        .inherits
        .iter()
        .filter(|edge| edge.sub_name == "Child")
        .map(|edge| edge.super_name.as_str())
        .collect();
    assert_eq!(bounds, vec![":", "First", "Second"], "{bounds:?}");
    let imports: Vec<&str> = parsed
        .imports
        .iter()
        .map(|item| item.module_path.as_str())
        .collect();
    assert_eq!(
        imports,
        vec!["crate::items::Alpha", "crate::items::Beta"],
        "{imports:?}"
    );
    let call = parsed
        .calls
        .iter()
        .find(|call| call.callee == "other::thing")
        .ok_or("expected an other::thing call")?;
    assert_eq!(
        call.arg_texts,
        vec!["first()".to_string(), "second()".to_string()],
        "{call:?}"
    );
    Ok(())
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse("fn ( { this is not valid rust @@@");
    // No panic is the assertion; symbols may or may not be empty
    // depending on tree-sitter's error recovery, which is fine.
    let _ = parsed;
}
