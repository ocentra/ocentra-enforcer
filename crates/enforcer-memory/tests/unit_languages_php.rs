use enforcer_memory::languages::php::parse;
use enforcer_memory::parsers::SymbolKind;

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_class_interface_function_symbols() {
    let src = r#"<?php
class UserController {}
interface Repo {}
function top_level() {}
"#;
    let parsed = parse(src);
    let names: Vec<&str> = parsed.symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"UserController"));
    assert!(names.contains(&"Repo"));
    assert!(names.contains(&"top_level"));
}

#[test]
fn extracts_method_and_tags_it_as_method_inside_class() {
    let src = "<?php class C { public function f() {} }";
    let parsed = parse(src);
    assert_eq!(symbol_kind(&parsed.symbols, "f"), Some(&SymbolKind::Method));
}

#[test]
fn extracts_namespace_as_module() {
    let src = "<?php namespace App\\Services; class C {}";
    let parsed = parse(src);
    assert!(parsed
        .symbols
        .iter()
        .any(|s| s.name == "App\\Services" && s.kind == SymbolKind::Module));
}

#[test]
fn extracts_use_imports() {
    let src = "<?php\nuse App\\Models\\User;\nuse App\\Bar as Baz;\nuse function App\\Helpers\\format_name;\n";
    let parsed = parse(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"App\\Models\\User"));
    assert!(paths.iter().any(|p| p.contains("App\\Bar")));
    assert!(paths.iter().any(|p| p.contains("format_name")));
}

#[test]
fn extracts_require_include_as_imports() {
    let src = r#"<?php
require 'config.php';
require_once 'bootstrap.php';
include 'helpers.php';
"#;
    let parsed = parse(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"config.php"));
    assert!(paths.contains(&"bootstrap.php"));
    assert!(paths.contains(&"helpers.php"));
}

#[test]
fn extracts_call_edges() {
    let src = "<?php function f() { helper($x); $this->bar(); Route::get('/x', 'y'); }";
    let parsed = parse(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"helper"));
    assert!(callees.contains(&"bar"));
    assert!(callees.iter().any(|c| c.ends_with("get")));
}

#[test]
fn extracts_extends_as_inherits() {
    let src = "<?php class C extends Base {}";
    let parsed = parse(src);
    assert!(parsed
        .inherits
        .iter()
        .any(|e| e.sub_name == "C" && e.super_name == "Base"));
}

#[test]
fn extracts_implements_edges() {
    let src = "<?php class C implements Countable, IteratorAggregate {}";
    let parsed = parse(src);
    assert!(parsed
        .implements
        .iter()
        .any(|e| e.trait_name == "Countable"));
    assert!(parsed
        .implements
        .iter()
        .any(|e| e.trait_name == "IteratorAggregate"));
}

#[test]
fn extracts_interface_extends_as_inherits() {
    let src = "<?php interface Repo extends Countable, Base {}";
    let parsed = parse(src);
    assert!(parsed
        .inherits
        .iter()
        .any(|e| e.sub_name == "Repo" && e.super_name == "Countable"));
    assert!(parsed
        .inherits
        .iter()
        .any(|e| e.sub_name == "Repo" && e.super_name == "Base"));
}

#[test]
fn extracts_php8_attribute_decorations() {
    let src = "<?php #[Something]\nclass C {}";
    let parsed = parse(src);
    assert!(parsed
        .decorates
        .iter()
        .any(|d| d.target_name == "C" && d.decorator_name == "Something"));
}

#[test]
fn detects_phpunit_test_via_attribute_name_and_test_case_extends() {
    let src = r#"<?php
class MyTest extends TestCase {
    public function testFoo() {}
    public function helper() {}
}
class Other {
    #[Test]
    public function checkThing() {}
    public function notATest() {}
}
"#;
    let parsed = parse(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "testFoo"),
        Some(&SymbolKind::Test)
    );
    // extends TestCase => every method in the class is a Test.
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Test)
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "checkThing"),
        Some(&SymbolKind::Test)
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "notATest"),
        Some(&SymbolKind::Method)
    );
}

#[test]
fn extracts_laravel_route_call() {
    let src = "<?php Route::get('/api/x', 'Controller@method');";
    let parsed = parse(src);
    assert!(parsed
        .routes
        .iter()
        .any(|r| r.method == "GET" && r.path == "/api/x"));
}

#[test]
fn extracts_symfony_route_attribute() {
    let src = r#"<?php
class C {
    #[Route("/users/{id}")]
    public function getUser() {}
}
"#;
    let parsed = parse(src);
    assert!(parsed.routes.iter().any(|r| r.path == "/users/{id}"));
}

/// Regression: iterator-based child traversal retains source order through
/// nested PHP attributes and call arguments.
#[test]
fn php_child_iteration_preserves_attribute_route_and_call_argument_order(
) -> Result<(), &'static str> {
    let src = r#"<?php
class C {
    #[Route("/first"), Route("/second")]
    public function save(int $first, string $second): Result {
        return Route::post("/first", [$first, $second]);
    }
}
"#;
    let parsed = parse(src);
    let routes: Vec<(&str, &str)> = parsed
        .routes
        .iter()
        .map(|route| (route.method.as_str(), route.path.as_str()))
        .collect();
    assert_eq!(
        routes,
        vec![("", "/first"), ("", "/second"), ("POST", "/first")],
        "{routes:?}"
    );

    let call = parsed
        .calls
        .iter()
        .find(|call| call.callee == "Route::post")
        .ok_or("expected a Route::post call")?;
    assert_eq!(
        call.arg_texts,
        vec!["\"/first\"".to_string(), "[$first, $second]".to_string()],
        "{call:?}"
    );
    Ok(())
}

#[test]
fn extracts_const_declaration_and_define_call_as_constants() {
    let src = r#"<?php
class C { const MAX = 10; }
define("APP_VERSION", "1.0");
"#;
    let parsed = parse(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "MAX"),
        Some(&SymbolKind::Constant)
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "APP_VERSION"),
        Some(&SymbolKind::Constant)
    );
}

#[test]
fn extracts_defines_edge_from_class_to_method() {
    let src = "<?php class C { public function f() {} }";
    let parsed = parse(src);
    assert!(parsed
        .defines
        .iter()
        .any(|d| d.container_name == "C" && d.member_name == "f"));
}

#[test]
fn extracts_type_refs_from_method_signature() {
    let src =
        "<?php class C { public function getUser(int $id, string $name): User { return null; } }";
    let parsed = parse(src);
    let refs: Vec<&str> = parsed
        .type_refs
        .iter()
        .filter(|t| t.from_name == "getUser")
        .map(|t| t.type_name.as_str())
        .collect();
    assert!(refs.contains(&"int"));
    assert!(refs.contains(&"string"));
    assert!(refs.contains(&"User"));
}

#[test]
fn extracts_named_closure_and_arrow_function_as_lambda() {
    let src = r#"<?php
$f = function($x) { return $x + 1; };
$g = fn($x) => $x * 2;
"#;
    let parsed = parse(src);
    assert_eq!(symbol_kind(&parsed.symbols, "f"), Some(&SymbolKind::Lambda));
    assert_eq!(symbol_kind(&parsed.symbols, "g"), Some(&SymbolKind::Lambda));
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = r#"<?php
namespace App;
use App\Models\User;
class C extends Base implements Countable {
    #[Route("/x")]
    public function f(int $a): void {}
}
"#;
    let first = parse(src);
    let second = parse(src);
    assert_eq!(first, second);
}
