use enforcer_memory::languages::python::parse;
use enforcer_memory::parsers::SymbolKind;

#[test]
fn preserves_decorated_class_and_call_children() {
    let parsed = parse(
        "@router.post(\"/items\")\nclass Child(Base):\n    def create(self, item):\n        return helper(item, \"ready\")\n",
    );

    let symbols: Vec<(&str, SymbolKind)> = parsed
        .symbols
        .iter()
        .map(|symbol| (symbol.name.as_str(), symbol.kind))
        .collect();
    let calls: Vec<(&str, Vec<&str>)> = parsed
        .calls
        .iter()
        .map(|call| {
            (
                call.callee.as_str(),
                call.arg_texts.iter().map(String::as_str).collect(),
            )
        })
        .collect();

    assert_eq!(
        symbols,
        vec![("Child", SymbolKind::Class), ("create", SymbolKind::Method)]
    );
    assert_eq!(parsed.routes.len(), 1);
    assert_eq!(parsed.routes[0].method, "POST");
    assert_eq!(parsed.routes[0].path, "/items");
    assert_eq!(
        calls,
        vec![
            ("router.post", vec!["\"/items\""]),
            ("helper", vec!["item", "\"ready\""]),
        ]
    );
}

#[test]
fn extracts_function_and_test_symbols() {
    let src = "def normal():\n    pass\n\ndef test_something():\n    pass\n";
    let parsed = parse(src);
    let names_kinds: Vec<(&str, SymbolKind)> = parsed
        .symbols
        .iter()
        .map(|s| (s.name.as_str(), s.kind))
        .collect();
    assert!(names_kinds.contains(&("normal", SymbolKind::Function)));
    assert!(names_kinds.contains(&("test_something", SymbolKind::Test)));
}

#[test]
fn extracts_class_as_class() {
    // X06 rich vocabulary: Python classes are now their own Class
    // label, not folded into the generic Type.
    let src = "class Foo:\n    pass\n";
    let parsed = parse(src);
    assert!(parsed
        .symbols
        .iter()
        .any(|s| s.name == "Foo" && s.kind == SymbolKind::Class));
}

#[test]
fn extracts_imports() {
    let src = "import os\nfrom typing import List\n";
    let parsed = parse(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"os"));
    assert!(paths.contains(&"typing"));
}

#[test]
fn extracts_call_edges() {
    let src = "def f():\n    helper()\n    ns.thing()\n";
    let parsed = parse(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"helper"));
    assert!(callees.contains(&"ns.thing"));
}

#[test]
fn extracts_flask_route_decorator() {
    let src = "@app.route(\"/hello\")\ndef hello():\n    pass\n";
    let parsed = parse(src);
    assert_eq!(parsed.routes.len(), 1);
    assert_eq!(parsed.routes[0].method, "GET");
    assert_eq!(parsed.routes[0].path, "/hello");
}

#[test]
fn extracts_fastapi_post_decorator() {
    let src = "@router.post(\"/items\")\ndef create():\n    pass\n";
    let parsed = parse(src);
    assert!(parsed
        .routes
        .iter()
        .any(|r| r.method == "POST" && r.path == "/items"));
}
