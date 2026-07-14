use enforcer_memory::code_graph::{CodeGraph, Manifest};
use enforcer_memory::languages::cpp::parse;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_cpp";

#[test]
fn extracts_class_method_and_free_function() {
    let src = r#"
class Shape {
public:
    Shape();
    virtual double area() const;
private:
    double width_;
};

double Shape::area() const {
    return width_ * width_;
}

void helper_fn() {}
"#;
    let parsed = parse(src, false);
    let kinds: Vec<(&str, SymbolKind)> = parsed
        .symbols
        .iter()
        .map(|s| (s.name.as_str(), s.kind))
        .collect();
    assert!(kinds.contains(&("Shape", SymbolKind::Class)));
    assert!(kinds.contains(&("helper_fn", SymbolKind::Function)));
    // The out-of-line `Shape::area` definition is a Method, not a bare
    // Function, and DEFINES from Shape -> area.
    assert!(kinds.contains(&("area", SymbolKind::Method)));
    assert!(parsed
        .defines
        .iter()
        .any(|d| d.container_name == "Shape" && d.member_name == "area"));
}

#[test]
fn extracts_inherits_edge_from_base_class_clause() {
    let src = "class Base {}; class Derived : public Base {};";
    let parsed = parse(src, false);
    assert!(parsed
        .inherits
        .iter()
        .any(|i| i.sub_name == "Derived" && i.super_name == "Base"));
}

#[test]
fn detects_abstract_class_as_interface() {
    let src = r#"
class Drawable {
public:
    virtual void draw() = 0;
    virtual ~Drawable() = default;
};
"#;
    let parsed = parse(src, false);
    let kinds: Vec<(&str, SymbolKind)> = parsed
        .symbols
        .iter()
        .map(|s| (s.name.as_str(), s.kind))
        .collect();
    assert!(kinds.contains(&("Drawable", SymbolKind::Interface)));
}

#[test]
fn extracts_namespace_as_module() {
    let src = "namespace geometry { class Point {}; }";
    let parsed = parse(src, false);
    let kinds: Vec<(&str, SymbolKind)> = parsed
        .symbols
        .iter()
        .map(|s| (s.name.as_str(), s.kind))
        .collect();
    assert!(kinds.contains(&("geometry", SymbolKind::Module)));
    assert!(kinds.contains(&("Point", SymbolKind::Class)));
}

#[test]
fn extracts_named_lambda_binding() {
    let src = "auto adder = [](int a, int b) { return a + b; };";
    let parsed = parse(src, false);
    let kinds: Vec<(&str, SymbolKind)> = parsed
        .symbols
        .iter()
        .map(|s| (s.name.as_str(), s.kind))
        .collect();
    assert!(kinds.contains(&("adder", SymbolKind::Lambda)));
}

#[test]
fn extracts_include_imports_and_calls() {
    let src = r#"
#include <vector>
#include "myheader.h"
void f() { helper(); other::thing(1, 2); }
"#;
    let parsed = parse(src, false);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"vector"));
    assert!(paths.contains(&"myheader.h"));
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"helper"));
    assert!(callees.contains(&"other::thing"));
}

/// Regression: iterator traversal retains written order through base clauses,
/// class fields, and nested call arguments.
#[test]
fn cpp_child_iteration_preserves_inheritance_fields_and_call_argument_order() -> TestResult {
    let src = r#"
class First {};
class Second {};
class Derived : public First, private Second {
    int first_field;
    int second_field;
};

void run() { other::thing(first(), second()); }
"#;
    let parsed = parse(src, false);
    let bases: Vec<&str> = parsed
        .inherits
        .iter()
        .filter(|edge| edge.sub_name == "Derived")
        .map(|edge| edge.super_name.as_str())
        .collect();
    assert_eq!(bases, vec!["First", "Second"], "{bases:?}");

    let fields: Vec<&str> = parsed
        .defines
        .iter()
        .filter(|edge| edge.container_name == "Derived")
        .map(|edge| edge.member_name.as_str())
        .collect();
    assert_eq!(fields, vec!["first_field", "second_field"], "{fields:?}");

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
fn detects_gtest_test_macro() {
    let src = r#"
TEST(MathSuite, AddsNumbers) {
    int result = 1 + 1;
}
TEST_F(FixtureSuite, DoesWork) {
}
"#;
    let parsed = parse(src, false);
    let names: Vec<&str> = parsed
        .symbols
        .iter()
        .filter(|s| s.kind == SymbolKind::Test)
        .map(|s| s.name.as_str())
        .collect();
    assert!(names.contains(&"MathSuite.AddsNumbers"));
    assert!(names.contains(&"FixtureSuite.DoesWork"));
}

#[test]
fn is_test_file_promotes_free_functions_and_methods_to_test() {
    let src = "void case_one() {} class Fixture { public: void case_two() {} };";
    let parsed = parse(src, true);
    assert!(parsed
        .symbols
        .iter()
        .filter(|s| s.name == "case_one" || s.name == "case_two")
        .all(|s| s.kind == SymbolKind::Test));
}

#[test]
fn extracts_typedef_and_using_alias() {
    let src = "typedef int MyInt; using StringAlias = std::string;";
    let parsed = parse(src, false);
    let kinds: Vec<(&str, SymbolKind)> = parsed
        .symbols
        .iter()
        .map(|s| (s.name.as_str(), s.kind))
        .collect();
    assert!(kinds.contains(&("MyInt", SymbolKind::TypeAlias)));
    assert!(kinds.contains(&("StringAlias", SymbolKind::TypeAlias)));
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse("class ( { this is not valid C++ @@@", false);
    let _ = parsed;
}

/// Complexity metrics hand case: a `switch` with two `case` arms inside
/// a `for` loop -- cyclomatic complexity 4 (base 1 + for + 2 cases),
/// loop_depth 1.
#[test]
fn complexity_hand_case_switch_in_loop() -> TestResult {
    use enforcer_memory::complexity::{compute, find_definition_node, NodeKindTable};
    use tree_sitter::Parser;

    let src = r#"
int classify(int n) {
    for (int i = 0; i < n; i++) {
        switch (i) {
            case 0:
                break;
            case 1:
                break;
        }
    }
    return n;
}
"#;
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_cpp::LANGUAGE.into())
        .map_err(|e| format!("grammar loads: {e}"))?;
    let tree = parser
        .parse(src, None)
        .ok_or_else(|| "parses".to_string())?;
    let table = NodeKindTable::cpp();
    let root = tree.root_node();
    let def_node = find_definition_node(root, "classify", 2, src.as_bytes(), &table)
        .ok_or_else(|| "finds classify's definition node".to_string())?;
    let metrics = compute(def_node, "classify", src.as_bytes(), &table);
    assert_eq!(metrics.complexity, 4);
    assert_eq!(metrics.loop_count, 1);
    assert_eq!(metrics.loop_depth, 1);
    Ok(())
}

fn run_git(dir: &Path, args: &[&str]) -> TestResult {
    let status = Command::new("git").args(args).current_dir(dir).status()?;
    if !status.success() {
        return Err(format!("git {args:?} failed").into());
    }
    Ok(())
}

fn init_repo(dir: &Path) -> TestResult {
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

fn copy_fixtures(dest: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture_root = manifest_dir.join(FIXTURE_DIR);
    let mut copied = Vec::new();
    for entry in fs::read_dir(&fixture_root)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            let dest_path = dest.join(entry.file_name());
            fs::copy(entry.path(), &dest_path)?;
            copied.push(dest_path);
        }
    }
    Ok(copied)
}

/// Incremental reindex over a real fixture repo, same pattern as
/// `unit_languages_c.rs`'s `c_fixture_repo_reindexes_incrementally`:
/// unchanged files skip re-parse, a changed C++ file gets a fresh
/// symbol set, and `CodeGraph` never panics walking a real C++ fixture
/// repo end-to-end (not just the standalone extractor).
#[test]
fn cpp_fixture_repo_reindexes_incrementally() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_repo(dir.path())?;
    let files = copy_fixtures(dir.path())?;
    commit_all(dir.path(), "initial cpp fixture import")?;

    let mut graph1 = CodeGraph::new();
    let (manifest_v1, report_v1) =
        graph1.index_repository(dir.path(), &files, &Manifest::default())?;
    assert_eq!(report_v1.added.len(), files.len());

    let symbol_names: Vec<&str> = graph1.symbol_nodes().map(|s| s.name.as_str()).collect();
    assert!(symbol_names.contains(&"Widget"), "{symbol_names:?}");
    assert!(symbol_names.contains(&"widgets"), "{symbol_names:?}");
    assert!(
        symbol_names.iter().any(|n| n.contains("WidgetSuite")),
        "{symbol_names:?}"
    );

    // Second run, nothing changed: every file is skipped.
    let mut graph2 = CodeGraph::new();
    let (manifest_v2, report_v2) = graph2.index_repository(dir.path(), &files, &manifest_v1)?;
    assert_eq!(report_v2.unchanged.len(), files.len());
    assert!(report_v2.changed.is_empty());

    // Mutate widget.cpp (adds a new function) and reindex again.
    let widget_cpp = dir.path().join("widget.cpp");
    let mut contents = fs::read_to_string(&widget_cpp)?;
    contents.push_str("\nvoid brand_new_fn() {}\n");
    fs::write(&widget_cpp, contents)?;
    commit_all(dir.path(), "change widget.cpp")?;

    let mut graph3 = CodeGraph::new();
    let (_manifest_v3, report_v3) = graph3.index_repository(dir.path(), &files, &manifest_v2)?;
    assert_eq!(report_v3.changed, vec!["widget.cpp".to_string()]);

    let symbol_names_v3: Vec<&str> = graph3.symbol_nodes().map(|s| s.name.as_str()).collect();
    assert!(
        symbol_names_v3.contains(&"brand_new_fn"),
        "{symbol_names_v3:?}"
    );
    Ok(())
}
