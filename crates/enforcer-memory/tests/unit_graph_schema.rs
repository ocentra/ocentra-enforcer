use enforcer_domain::memory_types::NodeLabel;
use enforcer_memory::code_graph::{CodeGraph, CodeNode, Manifest, TombstoneNode};
use enforcer_memory::graph_schema::{get_graph_schema, node_label};
use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::Command;

type TestResult = std::result::Result<(), Box<dyn Error>>;

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
fn empty_graph_has_no_labels_and_no_edge_types() {
    let graph = CodeGraph::new();
    let schema = get_graph_schema(&graph);
    assert!(schema.labels.is_empty());
    assert!(schema.edge_types.is_empty());
    assert_eq!(usize::from(schema.total_nodes()), 0);
    assert_eq!(usize::from(schema.total_edges()), 0);
}

#[test]
fn schema_counts_match_a_mixed_repository_exactly() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_git_repo(dir.path())?;

    let rust_path = dir.path().join("lib.rs");
    fs::write(
        &rust_path,
        "use std::fs;\nstruct Foo;\nfn helper() { fs::read(\"x\"); }\n#[test]\nfn a_test() {}\n",
    )?;
    let js_path = dir.path().join("server.js");
    fs::write(&js_path, "app.get(\"/health\", (req, res) => {});")?;
    let text_only_path = dir.path().join("NOTES.qux");
    fs::write(&text_only_path, "free text")?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(
        dir.path(),
        &[rust_path, js_path, text_only_path],
        &Manifest::default(),
    )?;

    let schema = get_graph_schema(&graph);

    let label = |name: &str| -> usize {
        schema
            .labels
            .iter()
            .find(|l| l.label == name)
            .map(|l| l.count.get())
            .unwrap_or(0)
    };
    // 2 real files + 1 TextOnly file = 3 File-ish nodes, split by
    // label: File=2 (lib.rs, server.js), TextOnly=1 (NOTES.qux).
    assert_eq!(label("File"), 2);
    assert_eq!(label("TextOnly"), 1);
    assert_eq!(label("Function"), 1, "helper");
    // X06 rich vocabulary: `struct Foo;` is now a Struct node, not a
    // generic Type.
    assert_eq!(label("Struct"), 1, "Foo");
    assert_eq!(label("Type"), 0, "no generic Type nodes in this fixture");
    assert_eq!(label("Test"), 1, "a_test");
    assert_eq!(label("Tombstone"), 0, "omitted at zero count");

    let edge = |name: &str| -> usize {
        schema
            .edge_types
            .iter()
            .find(|e| e.edge_type == name)
            .map(|e| e.count.get())
            .unwrap_or(0)
    };
    assert_eq!(edge("Imports"), 1);
    // 2 calls: Rust's `fs::read("x")` inside helper(), PLUS the JS
    // route file's `app.get(...)` call expression itself is ALSO
    // recorded as a plain CallEdge (callee "app.get") in addition to
    // being recognized as a RouteEdge -- the two extractions are
    // independent, not mutually exclusive (see
    // crate::languages::typescript's call_expression arm).
    assert_eq!(edge("Calls"), 2);
    assert_eq!(edge("Route"), 1);

    assert_eq!(usize::from(schema.total_nodes()), graph.nodes().len());
    assert_eq!(
        usize::from(schema.total_edges()),
        graph.imports().len() + graph.calls().len() + graph.routes().len()
    );
    Ok(())
}

#[test]
fn zero_count_labels_and_edge_types_are_never_emitted() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_git_repo(dir.path())?;
    let file_path = dir.path().join("plain.rs");
    fs::write(
        &file_path,
        "// no symbols, no imports, no calls, no routes\n",
    )?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

    let schema = get_graph_schema(&graph);
    assert_eq!(schema.labels.len(), 1);
    assert_eq!(schema.labels[0].label, "File");
    assert!(schema.edge_types.is_empty());
    Ok(())
}

#[test]
fn output_ordering_matches_the_baseline_descending_by_count() -> TestResult {
    // docs/plans/enforcer-selfhost-plan/refs/x06-baseline-tool-schemas.md
    // §3.2: the baseline's get_schema_impl orders node_labels/
    // edge_types by descending row count, not alphabetically. Three
    // functions, one struct (X06 rich vocabulary: `struct Foo;` is a
    // Struct node, not a generic Type), one file -- Function (3) must
    // sort before both File (1) and Struct (1), and the two count-1
    // labels must tie-break alphabetically (File < Struct).
    let dir = tempfile::tempdir()?;
    init_git_repo(dir.path())?;
    let file_path = dir.path().join("lib.rs");
    fs::write(&file_path, "struct Foo;\nfn a() {}\nfn b() {}\nfn c() {}\n")?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

    let schema = get_graph_schema(&graph);
    let labels: Vec<(&str, usize)> = schema
        .labels
        .iter()
        .map(|l| (l.label.as_str(), l.count.get()))
        .collect();
    assert_eq!(
        labels,
        vec![("Function", 3), ("File", 1), ("Struct", 1)],
        "Function (highest count) must sort first; File/Struct (tied at 1) tie-break alphabetically"
    );
    Ok(())
}

#[test]
fn node_label_covers_every_codenode_variant() {
    // Sanity: node_label never panics/falls through on any variant
    // this crate defines -- exercised indirectly by the mixed-repo
    // test above via Tombstone too.
    let tomb = CodeNode::Tombstone(TombstoneNode {
        id: "tomb:x.rs".to_owned(),
        rel_path: "x.rs".to_owned(),
        last_commit: None,
        change_count: 0.into(),
        prior_chunk_ids: Vec::new(),
    });
    assert_eq!(node_label(&tomb), NodeLabel::Tombstone);
}
