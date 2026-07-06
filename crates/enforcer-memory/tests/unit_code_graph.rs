use enforcer_memory::code_graph::{
    CodeGraph, CodeNode, IndexMode, IndexOptions, IndexWithOptionsError, Manifest, TombstoneNode,
};
use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::Command;

type TestResult = Result<(), Box<dyn Error>>;

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

#[test]
fn unchanged_file_is_skipped_on_second_run() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_git_repo(dir.path())?;
    let file_path = dir.path().join("a.rs");
    fs::write(&file_path, "fn a() {}")?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    let (manifest_v1, report_v1) = graph.index_repository(
        dir.path(),
        std::slice::from_ref(&file_path),
        &Manifest::default(),
    )?;
    assert_eq!(report_v1.added, vec!["a.rs".to_string()]);

    let mut graph2 = CodeGraph::new();
    let (_manifest_v2, report_v2) =
        graph2.index_repository(dir.path(), &[file_path], &manifest_v1)?;
    assert_eq!(report_v2.unchanged, vec!["a.rs".to_string()]);
    assert!(report_v2.changed.is_empty());
    assert!(report_v2.added.is_empty());
    Ok(())
}

#[test]
fn changed_file_is_reindexed() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_git_repo(dir.path())?;
    let file_path = dir.path().join("a.rs");
    fs::write(&file_path, "fn a() {}")?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    let (manifest_v1, _) = graph.index_repository(
        dir.path(),
        std::slice::from_ref(&file_path),
        &Manifest::default(),
    )?;

    fs::write(&file_path, "fn a() {} fn b() {}")?;
    commit_all(dir.path(), "second")?;

    let mut graph2 = CodeGraph::new();
    let (_manifest_v2, report_v2) =
        graph2.index_repository(dir.path(), &[file_path], &manifest_v1)?;
    assert_eq!(report_v2.changed, vec!["a.rs".to_string()]);
    let names: Vec<&str> = graph2.symbol_nodes().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"a"));
    assert!(names.contains(&"b"));
    Ok(())
}

#[test]
fn deleted_file_gets_tombstone_not_silently_dropped() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_git_repo(dir.path())?;
    let file_path = dir.path().join("a.rs");
    fs::write(&file_path, "fn a() {}")?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    let (manifest_v1, _) = graph.index_repository(
        dir.path(),
        std::slice::from_ref(&file_path),
        &Manifest::default(),
    )?;

    fs::remove_file(&file_path)?;
    commit_all(dir.path(), "delete a.rs")?;

    let mut graph2 = CodeGraph::new();
    let (manifest_v2, report_v2) = graph2.index_repository(dir.path(), &[], &manifest_v1)?;

    assert_eq!(report_v2.deleted, vec!["a.rs".to_string()]);
    let tombstones: Vec<&TombstoneNode> = graph2.tombstones().collect();
    assert_eq!(tombstones.len(), 1);
    assert_eq!(tombstones[0].rel_path, "a.rs");
    assert!(!tombstones[0].prior_chunk_ids.is_empty());
    assert!(!manifest_v2.entries.contains_key("a.rs"));
    Ok(())
}

#[test]
fn symbol_extraction_produces_function_type_test_nodes() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_git_repo(dir.path())?;
    let file_path = dir.path().join("lib.rs");
    fs::write(
        &file_path,
        "struct Foo;\nfn helper() {}\n#[test]\nfn a_test() {}\n",
    )?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

    let has_type = graph
        .nodes()
        .iter()
        .any(|n| matches!(n, CodeNode::Type(s) if s.name == "Foo"));
    let has_function = graph
        .nodes()
        .iter()
        .any(|n| matches!(n, CodeNode::Function(s) if s.name == "helper"));
    let has_test = graph
        .nodes()
        .iter()
        .any(|n| matches!(n, CodeNode::Test(s) if s.name == "a_test"));
    assert!(has_type, "expected a Type node for Foo");
    assert!(has_function, "expected a Function node for helper");
    assert!(has_test, "expected a Test node for a_test");
    Ok(())
}

#[test]
fn route_extraction_produces_route_edges() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_git_repo(dir.path())?;
    let file_path = dir.path().join("server.js");
    fs::write(&file_path, "app.get(\"/health\", (req, res) => {});")?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

    assert!(graph
        .routes()
        .iter()
        .any(|r| r.method == "GET" && r.path == "/health"));
    Ok(())
}

#[test]
fn import_and_call_edges_are_recorded() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_git_repo(dir.path())?;
    let file_path = dir.path().join("lib.rs");
    fs::write(&file_path, "use std::fs;\nfn f() { fs::read(\"x\"); }\n")?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

    assert!(graph.imports().iter().any(|i| i.module_path.contains("fs")));
    assert!(graph.calls().iter().any(|c| c.callee.contains("read")));
    Ok(())
}

#[test]
fn unsupported_extension_becomes_text_only_node_not_skipped() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_git_repo(dir.path())?;
    let file_path = dir.path().join("NOTES.qux");
    fs::write(&file_path, "some free text notes")?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

    let text_only = graph
        .nodes()
        .iter()
        .find(|n| matches!(n, CodeNode::TextOnly(f) if f.rel_path == "NOTES.qux"));
    assert!(
        text_only.is_some(),
        "unsupported extension must still produce a TextOnly node, never be skipped"
    );
    Ok(())
}

#[test]
fn fast_mode_skips_git_history_full_mode_computes_it() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_git_repo(dir.path())?;
    let file_path = dir.path().join("a.rs");
    fs::write(&file_path, "fn a() {}")?;
    commit_all(dir.path(), "first")?;

    let mut fast_graph = CodeGraph::new();
    fast_graph.index_repository_with_options(
        dir.path(),
        std::slice::from_ref(&file_path),
        &Manifest::default(),
        IndexOptions {
            mode: IndexMode::Fast,
            ..IndexOptions::default()
        },
    )?;
    let fast_file = fast_graph
        .file_nodes()
        .find(|f| f.rel_path == "a.rs")
        .ok_or("expected a.rs file node")?;
    assert_eq!(
        fast_file.last_commit, None,
        "fast mode must skip git history"
    );

    let mut full_graph = CodeGraph::new();
    full_graph.index_repository_with_options(
        dir.path(),
        std::slice::from_ref(&file_path),
        &Manifest::default(),
        IndexOptions {
            mode: IndexMode::Full,
            ..IndexOptions::default()
        },
    )?;
    let full_file = full_graph
        .file_nodes()
        .find(|f| f.rel_path == "a.rs")
        .ok_or("expected a.rs file node")?;
    assert!(
        full_file.last_commit.is_some(),
        "full mode must compute git history"
    );
    Ok(())
}

#[test]
fn persistence_true_without_project_name_is_a_typed_error() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_git_repo(dir.path())?;
    let file_path = dir.path().join("a.rs");
    fs::write(&file_path, "fn a() {}")?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    let outcome = graph.index_repository_with_options(
        dir.path(),
        std::slice::from_ref(&file_path),
        &Manifest::default(),
        IndexOptions {
            persistence: true,
            indexed_at: Some("2026-07-05T00:00:00Z"),
            ..IndexOptions::default()
        },
    );
    assert!(matches!(
        outcome,
        Err(IndexWithOptionsError::MissingProjectName)
    ));
    Ok(())
}

#[test]
fn persistence_true_writes_artifact_and_bootstrap_reimports_same_counts() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_git_repo(dir.path())?;
    let file_path = dir.path().join("a.rs");
    fs::write(&file_path, "fn a() {}\nfn b() {}\n")?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    graph.index_repository_with_options(
        dir.path(),
        std::slice::from_ref(&file_path),
        &Manifest::default(),
        IndexOptions {
            mode: IndexMode::Full,
            persistence: true,
            project_name: Some("demo"),
            indexed_at: Some("2026-07-05T00:00:00Z"),
        },
    )?;
    assert!(enforcer_memory::artifacts::artifact_exists(dir.path()));

    let original_node_count = graph.nodes().len();
    let original_edge_count = graph.imports().len() + graph.calls().len() + graph.routes().len();

    // A brand-new CodeGraph with an EMPTY previous manifest but the
    // artifact already on disk must bootstrap-import it before
    // indexing (even though this second call passes an empty file
    // list, so nothing would be found by walking otherwise).
    let mut bootstrapped = CodeGraph::new();
    bootstrapped.index_repository_with_options(
        dir.path(),
        &[],
        &Manifest::default(),
        IndexOptions::default(),
    )?;

    assert_eq!(bootstrapped.nodes().len(), original_node_count);
    let bootstrapped_edge_count =
        bootstrapped.imports().len() + bootstrapped.calls().len() + bootstrapped.routes().len();
    assert_eq!(bootstrapped_edge_count, original_edge_count);
    Ok(())
}

#[test]
fn index_repository_original_signature_still_compiles_and_runs() -> TestResult {
    // Back-compat: the plain 3-arg `index_repository` (every existing
    // call site in this crate) must keep working unchanged.
    let dir = tempfile::tempdir()?;
    init_git_repo(dir.path())?;
    let file_path = dir.path().join("a.rs");
    fs::write(&file_path, "fn a() {}")?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    let (_manifest, report) = graph.index_repository(
        dir.path(),
        std::slice::from_ref(&file_path),
        &Manifest::default(),
    )?;
    assert_eq!(report.added, vec!["a.rs".to_string()]);
    Ok(())
}
