use enforcer_memory::languages::rust::parse;
use enforcer_memory::parsers::SymbolKind;

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
fn extracts_struct_enum_trait_as_types() {
    let src = "struct Foo; enum Bar { A } trait Baz {}";
    let parsed = parse(src);
    let names: Vec<&str> = parsed.symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"Foo"));
    assert!(names.contains(&"Bar"));
    assert!(names.contains(&"Baz"));
    assert!(parsed
        .symbols
        .iter()
        .all(|s| s.kind == SymbolKind::Type || s.name.ends_with("_fn")));
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

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse("fn ( { this is not valid rust @@@");
    // No panic is the assertion; symbols may or may not be empty
    // depending on tree-sitter's error recovery, which is fine.
    let _ = parsed;
}
