//! X06 rich graph vocabulary -- hard tests for the additive
//! label/edge extension: per-language extraction of the new node
//! labels (Method/Class/Struct/Interface/Enum/TypeAlias/Module/Lambda/
//! Variable/Constant) and edge kinds (INHERITS/IMPLEMENTS/DECORATES/
//! TYPE_REF/DEFINES), plus `graph_schema` introspection and the DSL
//! MATCH query surface over the new vocabulary end-to-end.

use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::Command;

use enforcer_memory::analysis::query;
use enforcer_memory::analysis::CodeAdjacency;
use enforcer_memory::code_graph::{CodeGraph, CodeNode, Manifest};
use enforcer_memory::graph_schema::get_graph_schema;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn init_git_repo(dir: &Path) -> TestResult {
    run_git(dir, &["init", "--quiet"])?;
    run_git(dir, &["config", "user.email", "test@example.com"])?;
    run_git(dir, &["config", "user.name", "Test"])?;
    Ok(())
}

fn commit_all(dir: &Path, message: &str) -> TestResult {
    run_git(dir, &["add", "-A"])?;
    run_git(dir, &["commit", "--quiet", "-m", message])?;
    Ok(())
}

fn run_git(dir: &Path, args: &[&str]) -> TestResult {
    let status = Command::new("git").args(args).current_dir(dir).status()?;
    if !status.success() {
        return Err(format!("git {args:?} failed").into());
    }
    Ok(())
}

fn index_one_file(rel_name: &str, source: &str) -> TestResult<(tempfile::TempDir, CodeGraph)> {
    let dir = tempfile::tempdir()?;
    init_git_repo(dir.path())?;
    let file_path = dir.path().join(rel_name);
    fs::write(&file_path, source)?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;
    Ok((dir, graph))
}

// ---------------------------------------------------------------------
// Rust: labels
// ---------------------------------------------------------------------

#[test]
fn rust_extracts_full_label_set() -> TestResult {
    let src = r#"
mod inner {
    pub trait Greeter: std::fmt::Debug {
        fn greet(&self) -> String;
    }

    pub struct Widget;

    impl Greeter for Widget {
        fn greet(&self) -> String {
            "hi".to_string()
        }
    }

    pub enum Color { Red, Blue }

    pub type WidgetAlias = Widget;

    pub const MAX: i32 = 10;

    pub fn helper() {
        let closure = |x: i32| x + 1;
        let _ = closure(1);
    }
}
"#;
    let (_dir, graph) = index_one_file("lib.rs", src)?;

    let has = |pred: fn(&CodeNode) -> bool| graph.nodes().iter().any(pred);
    assert!(
        has(|n| matches!(n, CodeNode::Module(s) if s.name == "inner")),
        "expected Module node for `mod inner`"
    );
    assert!(
        has(|n| matches!(n, CodeNode::Interface(s) if s.name == "Greeter")),
        "expected Interface node for `trait Greeter`"
    );
    assert!(
        has(|n| matches!(n, CodeNode::Struct(s) if s.name == "Widget")),
        "expected Struct node for `struct Widget`"
    );
    assert!(
        has(|n| matches!(n, CodeNode::Method(s) if s.name == "greet")),
        "expected Method node for `greet` inside `impl Greeter for Widget`"
    );
    assert!(
        has(|n| matches!(n, CodeNode::Enum(s) if s.name == "Color")),
        "expected Enum node for `enum Color`"
    );
    assert!(
        has(|n| matches!(n, CodeNode::TypeAlias(s) if s.name == "WidgetAlias")),
        "expected TypeAlias node for `type WidgetAlias = Widget`"
    );
    assert!(
        has(|n| matches!(n, CodeNode::Constant(s) if s.name == "MAX")),
        "expected Constant node for `const MAX`"
    );
    assert!(
        has(|n| matches!(n, CodeNode::Function(s) if s.name == "helper")),
        "expected Function node for free-standing `helper`"
    );
    assert!(
        has(|n| matches!(n, CodeNode::Lambda(s) if s.name == "closure")),
        "expected Lambda node for `let closure = |x| ...`"
    );
    Ok(())
}

// ---------------------------------------------------------------------
// Rust: edges
// ---------------------------------------------------------------------

#[test]
fn rust_extracts_inherits_implements_defines_type_ref() -> TestResult {
    let src = r#"
trait Base {}
trait Sub: Base {}

struct Thing;

impl Sub for Thing {}

impl Thing {
    fn compute(&self, input: i32) -> i32 {
        input + 1
    }
}
"#;
    let (_dir, graph) = index_one_file("lib.rs", src)?;

    assert!(
        graph
            .inherits()
            .iter()
            .any(|e| e.super_name == "Base" && e.sub_id.contains("Sub")),
        "expected INHERITS edge Sub -> Base, got {:?}",
        graph.inherits()
    );
    assert!(
        graph
            .implements()
            .iter()
            .any(|e| e.trait_name == "Sub" && e.type_id.contains("Thing")),
        "expected IMPLEMENTS edge Thing -> Sub, got {:?}",
        graph.implements()
    );
    assert!(
        graph
            .defines()
            .iter()
            .any(|e| e.container_id.contains("Thing") && e.member_id.contains("compute")),
        "expected DEFINES edge Thing -> compute, got {:?}",
        graph.defines()
    );
    assert!(
        graph
            .type_refs()
            .iter()
            .any(|e| e.from_id.contains("compute") && e.type_name.contains("i32")),
        "expected TYPE_REF edge compute -> i32, got {:?}",
        graph.type_refs()
    );
    Ok(())
}

#[test]
fn rust_extracts_decorates_from_attribute_macro() -> TestResult {
    let src = "#[some_macro]\nfn decorated() {}\n";
    let (_dir, graph) = index_one_file("lib.rs", src)?;
    assert!(
        graph
            .decorates()
            .iter()
            .any(|e| e.decorator_name == "some_macro" && e.target_id.contains("decorated")),
        "expected DECORATES edge decorated -> some_macro, got {:?}",
        graph.decorates()
    );
    Ok(())
}

// ---------------------------------------------------------------------
// TypeScript: labels + edges
// ---------------------------------------------------------------------

#[test]
fn typescript_extracts_full_label_set() -> TestResult {
    let src = r#"
enum Color { Red, Blue }
type Alias = string;
namespace NS {}
const handler = (x: number) => x + 1;
class Base {}
interface Greetable {}
class Sub extends Base implements Greetable {
    greet(): void {}
}
"#;
    let (_dir, graph) = index_one_file("app.ts", src)?;

    let has = |pred: fn(&CodeNode) -> bool| graph.nodes().iter().any(pred);
    assert!(has(|n| matches!(n, CodeNode::Enum(s) if s.name == "Color")));
    assert!(has(
        |n| matches!(n, CodeNode::TypeAlias(s) if s.name == "Alias")
    ));
    assert!(has(|n| matches!(n, CodeNode::Module(s) if s.name == "NS")));
    assert!(has(
        |n| matches!(n, CodeNode::Lambda(s) if s.name == "handler")
    ));
    assert!(has(|n| matches!(n, CodeNode::Class(s) if s.name == "Sub")));
    assert!(has(
        |n| matches!(n, CodeNode::Interface(s) if s.name == "Greetable")
    ));
    assert!(has(
        |n| matches!(n, CodeNode::Method(s) if s.name == "greet")
    ));
    Ok(())
}

#[test]
fn typescript_extracts_extends_implements_and_decorator() -> TestResult {
    let src = r#"
class Base {}
interface Greetable {}
class Sub extends Base implements Greetable {}

@Injectable()
class Service {}
"#;
    let (_dir, graph) = index_one_file("app.ts", src)?;

    assert!(
        graph
            .inherits()
            .iter()
            .any(|e| e.super_name == "Base" && e.sub_id.contains("Sub")),
        "expected INHERITS edge Sub -> Base, got {:?}",
        graph.inherits()
    );
    assert!(
        graph
            .implements()
            .iter()
            .any(|e| e.trait_name == "Greetable" && e.type_id.contains("Sub")),
        "expected IMPLEMENTS edge Sub -> Greetable, got {:?}",
        graph.implements()
    );
    assert!(
        graph
            .decorates()
            .iter()
            .any(|e| e.decorator_name == "Injectable" && e.target_id.contains("Service")),
        "expected DECORATES edge Service -> Injectable, got {:?}",
        graph.decorates()
    );
    Ok(())
}

// ---------------------------------------------------------------------
// Python: labels + edges
// ---------------------------------------------------------------------

#[test]
fn python_extracts_full_label_set() -> TestResult {
    let src = r#"
class Base:
    pass

class Sub(Base):
    def method(self):
        pass

f = lambda x: x + 1

@some_decorator
def decorated():
    pass
"#;
    let (_dir, graph) = index_one_file("app.py", src)?;

    let has = |pred: fn(&CodeNode) -> bool| graph.nodes().iter().any(pred);
    assert!(has(|n| matches!(n, CodeNode::Class(s) if s.name == "Base")));
    assert!(has(|n| matches!(n, CodeNode::Class(s) if s.name == "Sub")));
    assert!(has(
        |n| matches!(n, CodeNode::Method(s) if s.name == "method")
    ));
    assert!(has(|n| matches!(n, CodeNode::Lambda(s) if s.name == "f")));
    assert!(has(
        |n| matches!(n, CodeNode::Function(s) if s.name == "decorated")
    ));
    Ok(())
}

#[test]
fn python_extracts_inheritance_and_decorator() -> TestResult {
    let src = r#"
class Base:
    pass

class Sub(Base):
    pass

@app.route("/x")
def handler():
    pass
"#;
    let (_dir, graph) = index_one_file("app.py", src)?;

    assert!(
        graph
            .inherits()
            .iter()
            .any(|e| e.super_name == "Base" && e.sub_id.contains("Sub")),
        "expected INHERITS edge Sub -> Base, got {:?}",
        graph.inherits()
    );
    assert!(
        graph
            .decorates()
            .iter()
            .any(|e| e.decorator_name.contains("route") && e.target_id.contains("handler")),
        "expected DECORATES edge handler -> app.route, got {:?}",
        graph.decorates()
    );
    Ok(())
}

// ---------------------------------------------------------------------
// graph_schema: new vocabulary is counted
// ---------------------------------------------------------------------

#[test]
fn graph_schema_counts_include_new_vocabulary() -> TestResult {
    let src = r#"
trait Base {}
struct Thing;
impl Base for Thing {}
"#;
    let (_dir, graph) = index_one_file("lib.rs", src)?;
    let schema = get_graph_schema(&graph);

    let label = |name: &str| -> usize {
        schema
            .labels
            .iter()
            .find(|l| l.label == name)
            .map(|l| l.count)
            .unwrap_or(0)
    };
    let edge = |name: &str| -> usize {
        schema
            .edge_types
            .iter()
            .find(|e| e.edge_type == name)
            .map(|e| e.count)
            .unwrap_or(0)
    };

    assert_eq!(label("Interface"), 1, "Base");
    assert_eq!(label("Struct"), 1, "Thing");
    assert_eq!(edge("IMPLEMENTS"), 1, "Thing implements Base");
    Ok(())
}

// ---------------------------------------------------------------------
// DSL: MATCH over the new vocabulary end-to-end
// ---------------------------------------------------------------------

#[test]
fn dsl_match_inherits_relationship_query_works() -> TestResult {
    let src = r#"
trait Base {}
trait Sub: Base {}
"#;
    let (_dir, graph) = index_one_file("lib.rs", src)?;
    let adjacency = CodeAdjacency::build(&graph);

    let parsed = query::parse("MATCH (c:Interface)-[:INHERITS]->(b:Interface) RETURN c, b")?;
    let rows = query::execute(&parsed, &adjacency, &graph)?;
    assert!(
        !rows.is_empty(),
        "expected at least one (Interface)-[:INHERITS]->(Interface) row"
    );
    assert!(rows.iter().any(|row| {
        row.get("c").is_some_and(|id| id.contains("Sub"))
            && row.get("b").is_some_and(|id| id.contains("Base"))
    }));
    Ok(())
}

#[test]
fn dsl_match_class_label_filter_works() -> TestResult {
    let src = "class Foo {}\nclass Bar {}\n";
    let (_dir, graph) = index_one_file("app.ts", src)?;
    let adjacency = CodeAdjacency::build(&graph);

    let parsed = query::parse("MATCH (c:Class) RETURN c")?;
    let rows = query::execute(&parsed, &adjacency, &graph)?;
    assert_eq!(
        rows.len(),
        2,
        "expected exactly two Class rows, got {rows:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------
// No regression: existing language extractor suites still model the
// pre-existing Function/Test/import/call/route behaviors (covered by
// unit_languages_{rust,typescript,python}.rs and unit_code_graph.rs,
// updated in this same change for the Struct/Class/Enum/Interface
// relabeling -- this suite additionally re-asserts the base shapes
// stay intact end-to-end through a full `index_repository` pass).
// ---------------------------------------------------------------------

#[test]
fn existing_function_test_import_call_route_shapes_still_work() -> TestResult {
    let src = "use std::fs;\nfn helper() { fs::read(\"x\"); }\n#[test]\nfn a_test() {}\n";
    let (_dir, graph) = index_one_file("lib.rs", src)?;

    assert!(graph
        .nodes()
        .iter()
        .any(|n| matches!(n, CodeNode::Function(s) if s.name == "helper")));
    assert!(graph
        .nodes()
        .iter()
        .any(|n| matches!(n, CodeNode::Test(s) if s.name == "a_test")));
    assert!(graph.imports().iter().any(|i| i.module_path.contains("fs")));
    assert!(graph.calls().iter().any(|c| c.callee.contains("read")));
    Ok(())
}
