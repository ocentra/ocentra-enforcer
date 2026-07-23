use enforcer_memory::code_graph::{CodeGraph, Manifest};
use enforcer_memory::languages::c::parse;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_c";

#[test]
fn extracts_function_and_struct_and_enum_and_typedef() {
    let src = r#"
struct Point { int x; int y; };
enum Color { RED, GREEN, BLUE };
typedef struct Point PointAlias;
typedef int MyInt;

int add(int a, int b) {
    return a + b;
}
"#;
    let parsed = parse(src, false);
    let kinds: Vec<(&str, SymbolKind)> = parsed
        .symbols
        .iter()
        .map(|s| (s.name.as_str(), s.kind))
        .collect();
    assert!(kinds.contains(&("Point", SymbolKind::Struct)));
    assert!(kinds.contains(&("Color", SymbolKind::Enum)));
    assert!(kinds.contains(&("PointAlias", SymbolKind::TypeAlias)));
    assert!(kinds.contains(&("MyInt", SymbolKind::TypeAlias)));
    assert!(kinds.contains(&("add", SymbolKind::Function)));
}

#[test]
fn extracts_define_value_macro_and_const_and_variable() {
    let src = r#"
#define MAX_SIZE 128
#define EMPTY_GUARD
const int kLimit = 10;
int counter = 0;
"#;
    let parsed = parse(src, false);
    let kinds: Vec<(&str, SymbolKind)> = parsed
        .symbols
        .iter()
        .map(|s| (s.name.as_str(), s.kind))
        .collect();
    assert!(kinds.contains(&("MAX_SIZE", SymbolKind::Constant)));
    assert!(!kinds.iter().any(|(n, _)| *n == "EMPTY_GUARD"));
    assert!(kinds.contains(&("kLimit", SymbolKind::Constant)));
    assert!(kinds.contains(&("counter", SymbolKind::Variable)));
}

#[test]
fn extracts_include_imports() {
    let src = r#"
#include <stdio.h>
#include "local_header.h"
"#;
    let parsed = parse(src, false);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"stdio.h"));
    assert!(paths.contains(&"local_header.h"));
}

#[test]
fn extracts_call_edges() {
    let src = "void f() { helper(); other_fn(1, 2); }";
    let parsed = parse(src, false);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"helper"));
    assert!(callees.contains(&"other_fn"));
}

/// Regression: iterator traversal retains written field, typedef, and call
/// argument ordering.
#[test]
fn c_child_iteration_preserves_fields_aliases_and_call_argument_order() -> Result<(), &'static str>
{
    let src = r#"
struct Pair { int first; int second; };
typedef int FirstAlias, SecondAlias;
void run() { other_fn(first(), second()); }
"#;
    let parsed = parse(src, false);
    let fields: Vec<&str> = parsed
        .defines
        .iter()
        .filter(|edge| edge.container_name == "Pair")
        .map(|edge| edge.member_name.as_str())
        .collect();
    assert_eq!(fields, vec!["first", "second"], "{fields:?}");
    let aliases: Vec<&str> = parsed
        .symbols
        .iter()
        .filter(|symbol| symbol.kind == SymbolKind::TypeAlias)
        .map(|symbol| symbol.name.as_str())
        .collect();
    assert_eq!(aliases, vec!["FirstAlias", "SecondAlias"], "{aliases:?}");
    let call = parsed
        .calls
        .iter()
        .find(|call| call.callee == "other_fn")
        .ok_or("expected an other_fn call")?;
    assert_eq!(
        call.arg_texts,
        vec!["first()".to_string(), "second()".to_string()],
        "{call:?}"
    );
    Ok(())
}

#[test]
fn detects_test_by_name_convention() {
    let src = "void test_addition() {} void teardown_test() {} void normal_fn() {}";
    let parsed = parse(src, false);
    let kinds: Vec<(&str, SymbolKind)> = parsed
        .symbols
        .iter()
        .map(|s| (s.name.as_str(), s.kind))
        .collect();
    assert!(kinds.contains(&("test_addition", SymbolKind::Test)));
    assert!(kinds.contains(&("teardown_test", SymbolKind::Test)));
    assert!(kinds.contains(&("normal_fn", SymbolKind::Function)));
}

#[test]
fn is_test_file_promotes_every_function_to_test() {
    let src = "void anything() {} void something_else() {}";
    let parsed = parse(src, true);
    assert!(parsed.symbols.iter().all(|s| s.kind == SymbolKind::Test));
}

#[test]
fn struct_defines_edges_to_field_members() {
    let src = "struct Vec3 { float x; float y; float z; };";
    let parsed = parse(src, false);
    let members: Vec<&str> = parsed
        .defines
        .iter()
        .filter(|d| d.container_name == "Vec3")
        .map(|d| d.member_name.as_str())
        .collect();
    assert!(members.contains(&"x"));
    assert!(members.contains(&"y"));
    assert!(members.contains(&"z"));
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse("int f( { this is not valid C @@@", false);
    let _ = parsed;
}

/// Complexity metrics smoke test: the baseline's own hand-verifiable
/// shape -- one `if`, one `for` loop nested inside it, so cyclomatic
/// complexity is 3 (base 1 + if + for) and loop_depth is 1.
#[test]
fn complexity_hand_case_if_and_for() -> TestResult {
    use enforcer_memory::complexity::{compute, find_definition_node, NodeKindTable};
    use tree_sitter::Parser;

    let src = r#"
int scan(int n) {
    if (n > 0) {
        for (int i = 0; i < n; i++) {
            n = n - 1;
        }
    }
    return n;
}
"#;
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_c::LANGUAGE.into())
        .map_err(|e| format!("grammar loads: {e}"))?;
    let tree = parser
        .parse(src, None)
        .ok_or_else(|| "parses".to_string())?;
    let table = NodeKindTable::c();
    let root = tree.root_node();
    let def_node = find_definition_node(root, "scan", 2, src.as_bytes(), &table)
        .ok_or_else(|| "finds scan's definition node".to_string())?;
    let metrics = compute(def_node, "scan", src.as_bytes(), &table);
    assert_eq!(metrics.complexity, 3);
    assert_eq!(metrics.loop_count, 1);
    assert_eq!(metrics.loop_depth, 1);
    assert_eq!(metrics.param_count, 1);
    Ok(())
}

/// Bonus proof: index a couple of real files from the C baseline
/// (`C:\Projects\codebase-memory-mcp\src`, read-only) and assert
/// nonzero functions+calls extracted -- the baseline indexing itself,
/// since the baseline tool is itself written in C.
#[test]
fn baseline_self_index_extracts_nonzero_functions_and_calls() {
    let candidates = [
        r"C:\Projects\codebase-memory-mcp\src\foundation\str_util.c",
        r"C:\Projects\codebase-memory-mcp\src\foundation\hash_table.c",
        r"C:\Projects\codebase-memory-mcp\src\foundation\arena.c",
    ];
    let mut total_functions = 0usize;
    let mut total_calls = 0usize;
    let mut files_read = 0usize;
    for path in candidates {
        let Ok(source) = fs::read_to_string(path) else {
            continue; // baseline repo not present in this environment; skip gracefully.
        };
        files_read += 1;
        let parsed = parse(&source, false);
        total_functions += parsed
            .symbols
            .iter()
            .filter(|s| matches!(s.kind, SymbolKind::Function | SymbolKind::Test))
            .count();
        total_calls += parsed.calls.len();
    }
    if files_read == 0 {
        // Baseline repo not checked out in this environment -- do not
        // fail the suite over an absent read-only external fixture.
        return;
    }
    assert!(
        total_functions > 0,
        "expected at least one function extracted from the C baseline"
    );
    assert!(
        total_calls > 0,
        "expected at least one call edge extracted from the C baseline"
    );
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

/// Incremental reindex over a real fixture repo (same tempdir-git
/// pattern as `unit_languages_go.rs`/`unit_languages_java.rs`):
/// unchanged files skip re-parse, a changed C file gets a fresh symbol
/// set, and `CodeGraph` never panics walking a real C fixture repo
/// end-to-end (not just the standalone extractor).
#[test]
fn c_fixture_repo_reindexes_incrementally() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_repo(dir.path())?;
    let files = copy_fixtures(dir.path())?;
    commit_all(dir.path(), "initial c fixture import")?;

    let mut graph1 = CodeGraph::new();
    let (manifest_v1, report_v1) =
        graph1.index_repository(dir.path(), &files, &Manifest::default())?;
    assert_eq!(report_v1.added.len(), files.len());

    let symbol_names: Vec<&str> = graph1.symbol_nodes().map(|s| s.name.as_str()).collect();
    assert!(symbol_names.contains(&"widget_new"));
    assert!(symbol_names.contains(&"Widget"));
    assert!(
        symbol_names.contains(&"test_widget_new_sets_id"),
        "{symbol_names:?}"
    );

    // Second run, nothing changed: every file is skipped.
    let mut graph2 = CodeGraph::new();
    let (manifest_v2, report_v2) = graph2.index_repository(dir.path(), &files, &manifest_v1)?;
    assert_eq!(report_v2.unchanged.len(), files.len());
    assert!(report_v2.changed.is_empty());

    // Mutate widget.c (adds a new function) and reindex again.
    let widget_c = dir.path().join("widget.c");
    let mut contents = fs::read_to_string(&widget_c)?;
    contents.push_str("\nvoid brand_new_fn() {}\n");
    fs::write(&widget_c, contents)?;
    commit_all(dir.path(), "change widget.c")?;

    let mut graph3 = CodeGraph::new();
    let (_manifest_v3, report_v3) = graph3.index_repository(dir.path(), &files, &manifest_v2)?;
    assert_eq!(report_v3.changed, vec!["widget.c".to_string()]);

    let symbol_names_v3: Vec<&str> = graph3.symbol_nodes().map(|s| s.name.as_str()).collect();
    assert!(
        symbol_names_v3.contains(&"brand_new_fn"),
        "{symbol_names_v3:?}"
    );
    Ok(())
}
