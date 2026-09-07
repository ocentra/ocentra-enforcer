//! Hard tests for the Go extractor ([`enforcer_syntax::languages::go`]):
//! symbol labels (function/method/struct/interface/typealias/const/
//! var/module), every edge kind Go supports (IMPORTS, CALLS, INHERITS
//! via embedded struct fields, TYPE_REF, DEFINES; IMPLEMENTS is
//! intentionally absent -- Go interface satisfaction is structural,
//! not a written clause), `_test.go`/`TestXxx` test detection, and
//! `net/http` route extraction.

use enforcer_domain::memory_types::ReceiverHint;
use enforcer_domain::memory_types::ResolutionConfidence;
use enforcer_memory::code_graph::{CodeGraph, Manifest};
use enforcer_memory::resolution::{self};
use enforcer_syntax::languages::go::parse;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_go";

#[test]
fn checked_child_traversal_preserves_go_import_call_and_type_output() {
    let parsed = parse(
        r#"package widget
import (
    "net/http"
    alias "example.com/alias"
)
type Widget struct { Name string; Embedded }
type Service interface { Run(http.Handler) }
func (w *Widget) Serve(h http.Handler) { http.HandleFunc("/health", w.Serve) }
"#,
        false,
    );

    let imports: Vec<&str> = parsed
        .imports
        .iter()
        .map(|entry| entry.module_path.as_str())
        .collect();
    assert_eq!(imports, vec!["net/http", "example.com/alias"]);
    assert!(parsed
        .symbols
        .iter()
        .any(|entry| entry.name == "Widget" && entry.kind == SymbolKind::Struct));
    assert!(parsed
        .symbols
        .iter()
        .any(|entry| entry.name == "Service" && entry.kind == SymbolKind::Interface));
    assert!(parsed
        .calls
        .iter()
        .any(|entry| entry.callee == "http.HandleFunc"));
    assert!(parsed
        .routes
        .iter()
        .any(|entry| entry.path == "/health" && entry.method == "ANY"));
}

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
    assert!(kinds.contains(&("Widget", SymbolKind::Struct)));
    assert!(
        kinds.contains(&("Drawable", SymbolKind::Interface)),
        "{kinds:?}"
    );
    assert!(kinds.contains(&("ID", SymbolKind::TypeAlias)));
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
    assert!(inherits.contains(&("Widget", "Base")));

    // Named field is DEFINES, not INHERITS.
    let defines: Vec<(&str, &str)> = parsed
        .defines
        .iter()
        .map(|d| (d.container_name.as_str(), d.member_name.as_str()))
        .collect();
    assert!(defines.contains(&("Widget", "Name")));
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
    assert!(defines.contains(&("Drawable", "Draw")));
    assert!(defines.contains(&("Drawable", "Resize")));
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
    assert!(defines.contains(&("Widget", "Draw")));
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
    assert!(paths.contains(&"fmt"));
    assert!(paths.contains(&"net/http"));
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
    assert!(callees.contains(&"helper"));
    assert!(callees.contains(&"fmt.Println"));
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
    assert!(types.contains(&"int"));
    assert!(types.contains(&"string"));
    assert!(types.contains(&"bool"));
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
    assert!(routes.contains(&("ANY", "/widgets")));
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
    assert!(routes.contains(&("GET", "/widgets")));
}

#[test]
fn call_inside_function_records_from_symbol_scope() -> TestResult {
    let src = r#"
package widget

func Render(w Widget) string {
	return w.Draw()
}
"#;
    let parsed = parse(src, false);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "w.Draw")
        .ok_or("expected a w.Draw call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("Render"), "{call:?}");
    assert_eq!(call.from_symbol_line, Some(4.into()), "{call:?}");
    Ok(())
}

#[test]
fn module_scope_call_has_no_from_symbol() -> TestResult {
    let src = r#"
package widget

var registry = makeRegistry()
"#;
    let parsed = parse(src, false);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "makeRegistry")
        .ok_or("expected a makeRegistry call")?;
    assert_eq!(call.from_symbol, None, "{call:?}");
    assert_eq!(call.from_symbol_line, None, "{call:?}");
    Ok(())
}

#[test]
fn method_call_records_identifier_receiver_and_args() -> TestResult {
    let src = r#"
package widget

func Render(w Widget, label string) string {
	return w.Draw(label, 42)
}
"#;
    let parsed = parse(src, false);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "w.Draw")
        .ok_or("expected a w.Draw call")?;
    assert_eq!(call.receiver_text.as_deref(), Some("w"), "{call:?}");
    assert_eq!(
        call.receiver_hint,
        Some(ReceiverHint::Identifier),
        "{call:?}"
    );
    assert_eq!(
        call.arg_texts,
        vec!["label".to_string(), "42".to_string()],
        "{call:?}"
    );
    Ok(())
}

#[test]
fn unqualified_call_has_no_receiver() -> TestResult {
    let src = r#"
package widget

func f() {
	helper()
}
"#;
    let parsed = parse(src, false);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.receiver_text, None, "{call:?}");
    assert_eq!(call.receiver_hint, None, "{call:?}");
    assert!(call.arg_texts.is_empty(), "{call:?}");
    Ok(())
}

#[test]
fn constructor_call_receiver_is_new_expression_hint() -> TestResult {
    // Go convention: NewXxx(...) is the constructor idiom, so a call on
    // its result gets the NewExpression receiver hint.
    let src = r#"
package widget

func f() string {
	return NewWidget().Draw()
}
"#;
    let parsed = parse(src, false);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "NewWidget().Draw")
        .ok_or("expected a NewWidget().Draw call")?;
    assert_eq!(
        call.receiver_text.as_deref(),
        Some("NewWidget()"),
        "{call:?}"
    );
    assert_eq!(
        call.receiver_hint,
        Some(ReceiverHint::NewExpression),
        "{call:?}"
    );
    Ok(())
}

#[test]
fn literal_receiver_is_literal_hint() -> TestResult {
    let src = r#"
package widget

func f() {
	"x".count()
}
"#;
    let parsed = parse(src, false);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee.ends_with(".count"))
        .ok_or("expected a literal-receiver call")?;
    assert_eq!(call.receiver_hint, Some(ReceiverHint::Literal), "{call:?}");
    Ok(())
}

/// End-to-end: a Go method call on a typed receiver (`w Widget` param,
/// `w.Draw()`) resolves type-aware through the indexed graph to the
/// Widget method, not just any same-name symbol.
#[test]
fn typed_receiver_method_call_resolves_type_aware() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_repo(dir.path())?;
    let file = dir.path().join("widget.go");
    fs::write(
        &file,
        r#"package widget

type Widget struct{ Name string }

func (w Widget) Draw() string { return w.Name }

func Render(w Widget) string {
	return w.Draw()
}
"#,
    )?;
    commit_all(dir.path(), "go resolution fixture")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(dir.path(), &[file], &Manifest::default())?;
    let resolved = resolution::resolve(&graph);

    let (_, resolution_result) = graph
        .calls()
        .iter()
        .zip(resolved.iter())
        .find(|(call, _)| call.callee == "w.Draw")
        .ok_or("expected a resolved entry for the w.Draw() call")?;
    assert!(
        matches!(
            resolution_result.confidence,
            ResolutionConfidence::Resolved | ResolutionConfidence::Probable
        ),
        "{resolution_result:?}"
    );
    assert_eq!(
        resolution_result.candidates.len(),
        1,
        "{resolution_result:?}"
    );
    assert!(
        resolution_result.candidates[0].contains("Draw"),
        "{resolution_result:?}"
    );
    assert!(
        resolution_result
            .from_symbol_id
            .as_deref()
            .is_some_and(|id| id.contains("Render")),
        "{resolution_result:?}"
    );
    Ok(())
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
    assert!(symbol_names.contains(&"Widget"));
    assert!(symbol_names.contains(&"NewWidget"));
    assert!(symbol_names.contains(&"TestNewWidget"));

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
