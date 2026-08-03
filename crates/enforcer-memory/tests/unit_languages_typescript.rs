use enforcer_syntax::languages::typescript::parse;
use enforcer_syntax::parsers::Language;

#[test]
fn extracts_function_class_interface_symbols() {
    let src = "function foo() {} class Bar {} interface Baz {}";
    let parsed = parse(src, Language::TypeScript);
    let names: Vec<&str> = parsed.symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"foo"));
    assert!(names.contains(&"Bar"));
    assert!(names.contains(&"Baz"));
}

#[test]
fn extracts_import_statements() {
    let src = "import { foo } from \"./foo\";\nimport bar from 'bar-pkg';";
    let parsed = parse(src, Language::TypeScript);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"./foo"));
    assert!(paths.contains(&"bar-pkg"));
}

#[test]
fn extracts_call_edges() {
    let src = "function f() { helper(); ns.thing(1); }";
    let parsed = parse(src, Language::JavaScript);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"helper"));
    assert!(callees.contains(&"ns.thing"));
}

#[test]
fn extracts_express_style_route() {
    let src = "app.get(\"/users/:id\", (req, res) => { res.send(1); });";
    let parsed = parse(src, Language::JavaScript);
    assert_eq!(parsed.routes.len(), 1);
    assert_eq!(parsed.routes[0].method, "GET");
    assert_eq!(parsed.routes[0].path, "/users/:id");
}

#[test]
fn extracts_nestjs_decorator_route() {
    let src = "class C { @Post(\"/items\") create() {} }";
    let parsed = parse(src, Language::TypeScript);
    assert!(parsed
        .routes
        .iter()
        .any(|r| r.method == "POST" && r.path == "/items"));
}

/// Regression: iterator traversal keeps written order across heritage,
/// decorators, and nested call arguments.
#[test]
fn typescript_child_iteration_preserves_heritage_route_and_call_argument_order(
) -> Result<(), &'static str> {
    let src = r#"
class Base {}
interface First {}
interface Second {}
class Controller extends Base implements First, Second {
    @Post("/items")
    create() { return api.save(first(), second()); }
}
"#;
    let parsed = parse(src, Language::TypeScript);
    let inherited: Vec<&str> = parsed
        .inherits
        .iter()
        .filter(|edge| edge.sub_name == "Controller")
        .map(|edge| edge.super_name.as_str())
        .collect();
    assert_eq!(inherited, vec!["Base"], "{inherited:?}");
    let implemented: Vec<&str> = parsed
        .implements
        .iter()
        .filter(|edge| edge.type_name == "Controller")
        .map(|edge| edge.trait_name.as_str())
        .collect();
    assert_eq!(implemented, vec!["First", "Second"], "{implemented:?}");
    assert!(parsed
        .routes
        .iter()
        .any(|route| route.method == "POST" && route.path == "/items"));

    let call = parsed
        .calls
        .iter()
        .find(|call| call.callee == "api.save")
        .ok_or("expected an api.save call")?;
    assert_eq!(
        call.arg_texts,
        vec!["first()".to_string(), "second()".to_string()],
        "{call:?}"
    );
    Ok(())
}
