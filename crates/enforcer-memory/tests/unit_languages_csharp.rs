use enforcer_memory::languages::csharp::parse;
use enforcer_memory::parsers::SymbolKind;

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_class_interface_struct_enum_symbols() {
    let src = r#"
public interface IRepo {}
public class UserController {}
public struct Point {}
public enum Status { Active, Inactive }
"#;
    let parsed = parse(src);
    let names: Vec<&str> = parsed.symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"IRepo"));
    assert!(names.contains(&"UserController"));
    assert!(names.contains(&"Point"));
    assert!(names.contains(&"Status"));
}

#[test]
fn extracts_method_and_tags_it_as_method_inside_class() {
    let src = "class C { public void F() {} }";
    let parsed = parse(src);
    assert_eq!(symbol_kind(&parsed.symbols, "F"), Some(&SymbolKind::Method));
}

#[test]
fn extracts_namespace_as_module() {
    let src = "namespace MyApp.Services { class C {} }";
    let parsed = parse(src);
    assert!(parsed
        .symbols
        .iter()
        .any(|s| s.name == "MyApp.Services" && s.kind == SymbolKind::Module));
}

#[test]
fn extracts_using_imports() {
    let src = "using System;\nusing System.Collections.Generic;\n";
    let parsed = parse(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"System"));
    assert!(paths.contains(&"System.Collections.Generic"));
}

#[test]
fn extracts_call_edges() {
    let src = "class C { void F() { helper(1); this.Bar(); } }";
    let parsed = parse(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"helper"));
    assert!(callees.iter().any(|c| c.ends_with("Bar")));
}

#[test]
fn extracts_inherits_and_implements_from_base_list() {
    let src = "class UserController : ControllerBase, IDisposable {}";
    let parsed = parse(src);
    assert!(parsed
        .inherits
        .iter()
        .any(|e| e.sub_name == "UserController" && e.super_name == "ControllerBase"));
    assert!(parsed
        .implements
        .iter()
        .any(|e| e.type_name == "UserController" && e.trait_name == "IDisposable"));
}

#[test]
fn extracts_implements_only_when_base_is_interface_shaped() {
    // A base-list whose FIRST entry already looks like an interface
    // (I + uppercase) means there is no base class at all -- every
    // entry in the list is IMPLEMENTS.
    let src = "class Repo : IRepo, IDisposable {}";
    let parsed = parse(src);
    assert!(parsed.inherits.is_empty());
    assert!(parsed.implements.iter().any(|e| e.trait_name == "IRepo"));
    assert!(parsed
        .implements
        .iter()
        .any(|e| e.trait_name == "IDisposable"));
}

#[test]
fn extracts_interface_extends_as_inherits() {
    let src = "interface IRepo : IDisposable, IBase {}";
    let parsed = parse(src);
    assert!(parsed
        .inherits
        .iter()
        .any(|e| e.sub_name == "IRepo" && e.super_name == "IDisposable"));
    assert!(parsed
        .inherits
        .iter()
        .any(|e| e.sub_name == "IRepo" && e.super_name == "IBase"));
}

#[test]
fn extracts_attribute_decorations() {
    let src = "[ApiController]\npublic class UserController {}";
    let parsed = parse(src);
    assert!(parsed
        .decorates
        .iter()
        .any(|d| d.target_name == "UserController" && d.decorator_name == "ApiController"));
}

#[test]
fn detects_nunit_xunit_mstest_test_attributes() {
    let src = r#"
class T {
    [Test] public void A() {}
    [Fact] public void B() {}
    [TestMethod] public void C() {}
    public void NotATest() {}
}
"#;
    let parsed = parse(src);
    for name in ["A", "B", "C"] {
        assert_eq!(
            symbol_kind(&parsed.symbols, name),
            Some(&SymbolKind::Test),
            "{name} should be Test"
        );
    }
    assert_eq!(
        symbol_kind(&parsed.symbols, "NotATest"),
        Some(&SymbolKind::Method)
    );
}

#[test]
fn extracts_http_attribute_route() {
    let src = r#"
class C {
    [HttpGet("/users/{id}")]
    public void GetUser() {}
}
"#;
    let parsed = parse(src);
    assert!(parsed
        .routes
        .iter()
        .any(|r| r.method == "GET" && r.path == "/users/{id}"));
}

#[test]
fn extracts_minimal_api_map_route() {
    let src = r#"app.MapGet("/users/{id}", () => {});"#;
    let parsed = parse(src);
    assert!(parsed
        .routes
        .iter()
        .any(|r| r.method == "GET" && r.path == "/users/{id}"));
}

#[test]
fn extracts_const_and_static_readonly_fields_as_constants() {
    let src = r#"
class C {
    const int MaxCount = 10;
    static readonly string Prefix = "x";
    public int NotConst = 0;
}
"#;
    let parsed = parse(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "MaxCount"),
        Some(&SymbolKind::Constant)
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Prefix"),
        Some(&SymbolKind::Constant)
    );
    assert_ne!(
        symbol_kind(&parsed.symbols, "NotConst"),
        Some(&SymbolKind::Constant)
    );
}

#[test]
fn extracts_defines_edge_from_class_to_method() {
    let src = "class C { void F() {} }";
    let parsed = parse(src);
    assert!(parsed
        .defines
        .iter()
        .any(|d| d.container_name == "C" && d.member_name == "F"));
}

#[test]
fn extracts_type_refs_from_method_signature() {
    let src = "class C { public User GetUser(int id, string name) { return null; } }";
    let parsed = parse(src);
    let refs: Vec<&str> = parsed
        .type_refs
        .iter()
        .filter(|t| t.from_name == "GetUser")
        .map(|t| t.type_name.as_str())
        .collect();
    assert!(refs.contains(&"int"));
    assert!(refs.contains(&"string"));
    assert!(refs.contains(&"User"));
}

#[test]
fn extracts_named_local_function_as_lambda() {
    let src = "class C { void Outer() { int Add(int a, int b) => a + b; } }";
    let parsed = parse(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Add"),
        Some(&SymbolKind::Lambda)
    );
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = r#"
using System;
namespace App {
    [ApiController]
    public class C : Base, IDisposable {
        [HttpGet("/x")]
        public void F(int a) {}
    }
}
"#;
    let first = parse(src);
    let second = parse(src);
    assert_eq!(first, second);
}
