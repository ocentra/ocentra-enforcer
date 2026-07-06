//! Hard tests for the Go extractor ([`enforcer_memory::languages::go`]):
//! symbol labels (function/method/struct/interface/typealias/const/
//! var/module), every edge kind Go supports (IMPORTS, CALLS, INHERITS
//! via embedded struct fields, TYPE_REF, DEFINES; IMPLEMENTS is
//! intentionally absent -- Go interface satisfaction is structural,
//! not a written clause), `_test.go`/`TestXxx` test detection, and
//! `net/http` route extraction.

use enforcer_memory::code_graph::{CodeGraph, Manifest};
use enforcer_memory::languages::go::parse;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_go";

#[test]
fn extracts_package_as_module_symbol() {
    let parsed = parse("package widget\n", false);
    let names_kinds: Vec<(&str, SymbolKind)> = parsed
        .symbols
        .iter()
        .map(|s| (s.name.as_str(), s.kind))
        .collect();
    assert!(names_kinds.contains(&("widget", SymbolKind::Module)));
}

#[test]
fn extracts_function_and_method_with_distinct_kinds() {
    let src = r#"
package widget

func NewWidget() {}

func (w *Widget) Draw() string { return "x" }
"#;
    let parsed = parse(src, false);
    let names_kinds: Vec<(&str, SymbolKind)> = parsed
        .symbols
        .iter()
        .map(|s| (s.name.as_str(), s.kind))
        .collect();
    assert!(names_kinds.contains(&("NewWidget", SymbolKind::Function)));
    assert!(names_kinds.contains(&("Draw", SymbolKind::Method)));
}

#[test]
fn extracts_struct_interface_typealias_const_var() {
    let src = r#"
package widget

type Widget struct {
	Name string
}

type Drawable interface {
	Draw() string
}

type ID = int

const MaxWidgets = 10

var registry = 0
"#;
    let parsed = parse(src, false);
    let kinds: Vec<(&str, SymbolKind)> = parsed
        .symbols
        .iter()
        .map(|s| (s.name.as_str(), s.kind))
        .collect();
    assert!(kinds.contains(&("Widget", SymbolKind::Struct)), "{kinds:?}");
    assert!(
        kinds.contains(&("Drawable", SymbolKind::Interface)),
        "{kinds:?}"
    );
    assert!(kinds.contains(&("ID", SymbolKind::TypeAlias)), "{kinds:?}");
    assert!(
        kinds.contains(&("MaxWidgets", SymbolKind::Constant)),
        "{kinds:?}"
    );
    assert!(
        kinds.contains(&("registry", SymbolKind::Variable)),
        "{kinds:?}"
    );
}

#[test]
fn extracts_embedded_field_as_inherits() {
    // Base is embedded (no field name) into Widget -- best-effort
    // INHERITS source for Go's structural composition idiom.
    let src = r#"
package widget

type Base struct {
	ID int
}

type Widget struct {
	Base
	Name string
}
"#;
    let parsed = parse(src, false);
    let inherits: Vec<(&str, &str)> = parsed
        .inherits
        .iter()
        .map(|i| (i.sub_name.as_str(), i.super_name.as_str()))
        .collect();
    assert!(inherits.contains(&("Widget", "Base")), "{inherits:?}");

    // Named field is DEFINES, not INHERITS.
    let defines: Vec<(&str, &str)> = parsed
        .defines
        .iter()
        .map(|d| (d.container_name.as_str(), d.member_name.as_str()))
        .collect();
    assert!(defines.contains(&("Widget", "Name")), "{defines:?}");
}

#[test]
fn extracts_interface_methods_as_defines() {
    let src = r#"
package widget

type Drawable interface {
	Draw() string
	Resize(w int, h int)
}
"#;
    let parsed = parse(src, false);
    let defines: Vec<(&str, &str)> = parsed
        .defines
        .iter()
        .map(|d| (d.container_name.as_str(), d.member_name.as_str()))
        .collect();
    assert!(defines.contains(&("Drawable", "Draw")), "{defines:?}");
    assert!(defines.contains(&("Drawable", "Resize")), "{defines:?}");
}

#[test]
fn extracts_method_receiver_as_defines() {
    let src = r#"
package widget

type Widget struct{ Name string }

func (w *Widget) Draw() string { return w.Name }
"#;
    let parsed = parse(src, false);
    let defines: Vec<(&str, &str)> = parsed
        .defines
        .iter()
        .map(|d| (d.container_name.as_str(), d.member_name.as_str()))
        .collect();
    assert!(defines.contains(&("Widget", "Draw")), "{defines:?}");
}

#[test]
fn extracts_imports() {
    let src = r#"
package widget

import (
	"fmt"
	"net/http"
)
"#;
    let parsed = parse(src, false);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"fmt"), "{paths:?}");
    assert!(paths.contains(&"net/http"), "{paths:?}");
}

#[test]
fn extracts_call_edges() {
    let src = r#"
package widget

func f() {
	helper()
	fmt.Println("x")
}
"#;
    let parsed = parse(src, false);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"helper"), "{callees:?}");
    assert!(callees.contains(&"fmt.Println"), "{callees:?}");
}

#[test]
fn extracts_signature_type_refs() {
    let src = r#"
package widget

func Combine(a int, b string) bool { return true }
"#;
    let parsed = parse(src, false);
    let types: Vec<&str> = parsed
        .type_refs
        .iter()
        .map(|t| t.type_name.as_str())
        .collect();
    assert!(types.contains(&"int"), "{types:?}");
    assert!(types.contains(&"string"), "{types:?}");
    assert!(types.contains(&"bool"), "{types:?}");
}

#[test]
fn test_file_detects_testxxx_functions_only() {
    let src = r#"
package widget

import "testing"

func TestNewWidget(t *testing.T) {}

func helperNotATest() {}
"#;
    let parsed = parse(src, true);
    let names_kinds: Vec<(&str, SymbolKind)> = parsed
        .symbols
        .iter()
        .map(|s| (s.name.as_str(), s.kind))
        .collect();
    assert!(
        names_kinds.contains(&("TestNewWidget", SymbolKind::Test)),
        "{names_kinds:?}"
    );
    assert!(
        names_kinds.contains(&("helperNotATest", SymbolKind::Function)),
        "{names_kinds:?}"
    );
}

#[test]
fn non_test_file_never_classifies_testxxx_as_test() {
    // Test detection is filename-gated (per Go convention): a
    // `TestXxx`-shaped function outside a `_test.go` file is a normal
    // function, not a test.
    let src = r#"
package widget

func TestLooking() {}
"#;
    let parsed = parse(src, false);
    let names_kinds: Vec<(&str, SymbolKind)> = parsed
        .symbols
        .iter()
        .map(|s| (s.name.as_str(), s.kind))
        .collect();
    assert!(
        names_kinds.contains(&("TestLooking", SymbolKind::Function)),
        "{names_kinds:?}"
    );
}

#[test]
fn extracts_net_http_handlefunc_route() {
    let src = r#"
package widget

import "net/http"

func RegisterRoutes(mux *http.ServeMux) {
	mux.HandleFunc("/widgets", ListWidgets)
}
"#;
    let parsed = parse(src, false);
    let routes: Vec<(&str, &str)> = parsed
        .routes
        .iter()
        .map(|r| (r.method.as_str(), r.path.as_str()))
        .collect();
    assert!(routes.contains(&("ANY", "/widgets")), "{routes:?}");
}

#[test]
fn extracts_mux_style_verb_route() {
    let src = r#"
package widget

func RegisterRoutes(router *Router) {
	router.GET("/widgets", ListWidgets)
}
"#;
    let parsed = parse(src, false);
    let routes: Vec<(&str, &str)> = parsed
        .routes
        .iter()
        .map(|r| (r.method.as_str(), r.path.as_str()))
        .collect();
    assert!(routes.contains(&("GET", "/widgets")), "{routes:?}");
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse("package ( { this is not valid go @@@", false);
    let _ = parsed;
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

/// Incremental reindex, reusing the same tempdir-git pattern as
/// `tests/code_graph_indexer.rs`: unchanged files skip re-parse, a
/// changed Go file gets a fresh symbol set, and CodeGraph never panics
/// walking a real Go fixture repo end-to-end (not just the standalone
/// extractor).
#[test]
fn go_fixture_repo_reindexes_incrementally() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_repo(dir.path())?;
    let files = copy_fixtures(dir.path())?;
    commit_all(dir.path(), "initial go fixture import")?;

    let mut graph1 = CodeGraph::new();
    let (manifest_v1, report_v1) =
        graph1.index_repository(dir.path(), &files, &Manifest::default())?;
    assert_eq!(report_v1.added.len(), files.len());

    let symbol_names: Vec<&str> = graph1.symbol_nodes().map(|s| s.name.as_str()).collect();
    assert!(symbol_names.contains(&"Widget"), "{symbol_names:?}");
    assert!(symbol_names.contains(&"NewWidget"), "{symbol_names:?}");
    assert!(symbol_names.contains(&"TestNewWidget"), "{symbol_names:?}");

    // Second run, nothing changed: every file is skipped.
    let mut graph2 = CodeGraph::new();
    let (manifest_v2, report_v2) = graph2.index_repository(dir.path(), &files, &manifest_v1)?;
    assert_eq!(report_v2.unchanged.len(), files.len());
    assert!(report_v2.changed.is_empty());

    // Mutate widget.go (adds a new function) and reindex again.
    let widget_go = dir.path().join("widget.go");
    let mut contents = fs::read_to_string(&widget_go)?;
    contents.push_str("\nfunc BrandNewFn() {}\n");
    fs::write(&widget_go, contents)?;
    commit_all(dir.path(), "change widget.go")?;

    let mut graph3 = CodeGraph::new();
    let (_manifest_v3, report_v3) = graph3.index_repository(dir.path(), &files, &manifest_v2)?;
    assert_eq!(report_v3.changed, vec!["widget.go".to_string()]);

    let symbol_names_v3: Vec<&str> = graph3.symbol_nodes().map(|s| s.name.as_str()).collect();
    assert!(
        symbol_names_v3.contains(&"BrandNewFn"),
        "{symbol_names_v3:?}"
    );
    Ok(())
}
