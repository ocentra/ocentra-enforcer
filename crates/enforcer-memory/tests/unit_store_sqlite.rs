use enforcer_memory::artifacts::{GraphSnapshot, GraphSymbolKindSnapshot};
use enforcer_memory::code_graph::{CodeGraph, CodeNode, Manifest};
use enforcer_memory::error::Result;
use enforcer_memory::ids::repo_root;
use enforcer_memory::log::read_verified;
use enforcer_memory::schema::{GraphEventKind, GraphEventLogEntry, SCHEMA_VERSION};
use enforcer_memory::store::sqlite::OperationalGraph;
use enforcer_memory::store::Store;
use std::path::Path;
use std::process::Command;

fn node_entry(seq: u64, id: &str, kind: &str) -> GraphEventLogEntry {
    GraphEventLogEntry {
        schema_version: SCHEMA_VERSION,
        seq,
        id: format!("evt-{seq}"),
        event: GraphEventKind::NodeAdded {
            node_id: id.to_owned(),
            node_kind: kind.to_owned(),
        },
        ts: "2026-07-04T00:00:00Z".to_owned(),
        supersedes_seq: None,
    }
}

fn edge_entry(seq: u64, from: &str, to: &str, label: &str) -> GraphEventLogEntry {
    GraphEventLogEntry {
        schema_version: SCHEMA_VERSION,
        seq,
        id: format!("evt-{seq}"),
        event: GraphEventKind::EdgeAdded {
            from: from.to_owned(),
            to: to.to_owned(),
            label: label.to_owned(),
        },
        ts: "2026-07-04T00:00:00Z".to_owned(),
        supersedes_seq: None,
    }
}

fn node_kind(node: &CodeNode) -> &'static str {
    match node {
        CodeNode::File(_) => "file",
        CodeNode::Function(_) => "function",
        CodeNode::Type(_) => "type",
        CodeNode::Test(_) => "test",
        CodeNode::TextOnly(_) => "text_only",
        CodeNode::Tombstone(_) => "tombstone",
        CodeNode::Method(_) => "method",
        CodeNode::Class(_) => "class",
        CodeNode::Struct(_) => "struct",
        CodeNode::Interface(_) => "interface",
        CodeNode::Enum(_) => "enum",
        CodeNode::TypeAlias(_) => "type_alias",
        CodeNode::Module(_) => "module",
        CodeNode::Lambda(_) => "lambda",
        CodeNode::Variable(_) => "variable",
        CodeNode::Constant(_) => "constant",
    }
}

fn run_git(repo: &Path, args: &[&str]) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let status = Command::new("git").args(args).current_dir(repo).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("git {} failed with {status}", args.join(" ")).into())
    }
}

#[test]
fn apply_and_counts() -> Result<()> {
    let mut graph = OperationalGraph::open_in_memory()?;
    graph.apply(&node_entry(0, "a", "file"))?;
    graph.apply(&node_entry(1, "b", "file"))?;
    graph.apply(&edge_entry(2, "a", "b", "imports"))?;
    assert_eq!(graph.node_count()?, 2);
    assert_eq!(graph.edge_count()?, 1);
    Ok(())
}

#[test]
fn rebuild_is_deterministic() -> Result<()> {
    let entries = vec![
        node_entry(0, "a", "file"),
        node_entry(1, "b", "file"),
        edge_entry(2, "a", "b", "imports"),
        node_entry(3, "c", "symbol"),
    ];
    let mut first = OperationalGraph::open_in_memory()?;
    first.rebuild(&entries)?;
    let mut second = OperationalGraph::open_in_memory()?;
    second.rebuild(&entries)?;
    assert_eq!(first.nodes_snapshot()?, second.nodes_snapshot()?);
    assert_eq!(first.node_count()?, second.node_count()?);
    assert_eq!(first.edge_count()?, second.edge_count()?);

    // Rebuilding a THIRD time by replaying twice into the same
    // database (idempotent apply) must not change the counts.
    first.rebuild(&entries)?;
    assert_eq!(first.node_count()?, 3);
    assert_eq!(first.edge_count()?, 1);
    Ok(())
}

#[test]
fn later_seq_supersedes_node_kind_for_the_same_id() -> Result<()> {
    let mut graph = OperationalGraph::open_in_memory()?;
    graph.apply(&node_entry(0, "a", "file"))?;
    graph.apply(&node_entry(1, "a", "symbol"))?;
    let snapshot = graph.nodes_snapshot()?;
    assert_eq!(snapshot, vec![("a".to_owned(), "symbol".to_owned())]);
    Ok(())
}

#[test]
fn store_backed_projection_rebuilds_from_a_real_code_graph_fixture(
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let repo_dir = tempfile::tempdir()?;
    run_git(repo_dir.path(), &["init", "--quiet"])?;
    run_git(
        repo_dir.path(),
        &["config", "user.email", "test@example.com"],
    )?;
    run_git(repo_dir.path(), &["config", "user.name", "Test"])?;

    let file_path = repo_dir.path().join("lib.rs");
    std::fs::write(
        &file_path,
        r#"
struct Widget;
enum Mode { Fast }
type WidgetAlias = Widget;
const LIMIT: usize = 1;

impl Widget {
    fn method(&self) {}
}

fn alpha() {}
fn beta() { alpha(); }

#[test]
fn beta_test() { beta(); }
"#,
    )?;
    run_git(repo_dir.path(), &["add", "-A"])?;
    run_git(repo_dir.path(), &["commit", "--quiet", "-m", "fixture"])?;

    let mut graph = CodeGraph::new();
    graph.index_repository(repo_dir.path(), &[file_path], &Manifest::default())?;
    let snapshot = GraphSnapshot::from_code_graph(&graph);
    for (name, kind) in [
        ("Widget", GraphSymbolKindSnapshot::Struct),
        ("Mode", GraphSymbolKindSnapshot::Enum),
        ("WidgetAlias", GraphSymbolKindSnapshot::TypeAlias),
        ("LIMIT", GraphSymbolKindSnapshot::Constant),
        ("method", GraphSymbolKindSnapshot::Method),
        ("alpha", GraphSymbolKindSnapshot::Function),
        ("beta_test", GraphSymbolKindSnapshot::Test),
    ] {
        assert!(
            snapshot
                .symbols
                .iter()
                .any(|symbol| symbol.name == name && symbol.kind == kind),
            "snapshot must preserve {name} as {kind:?}"
        );
    }

    let stores_dir = tempfile::tempdir()?;
    let repo_root = repo_root(&repo_dir.path().to_string_lossy())?;
    let mut store = Store::init(stores_dir.path(), &repo_root, "2026-07-07T00:00:00Z")?;
    let sqlite_path = store.sqlite_path();
    let log_path = store.graph_event_log_path();

    for node in graph.nodes() {
        let node_id = node.id().to_string();
        let node_kind = node_kind(node).to_owned();
        store
            .graph_event_log_mut()
            .append_with_seq(|seq| GraphEventLogEntry {
                schema_version: SCHEMA_VERSION,
                seq,
                id: format!("evt-node-{seq}"),
                event: GraphEventKind::NodeAdded {
                    node_id: node_id.clone(),
                    node_kind: node_kind.clone(),
                },
                ts: "2026-07-07T00:00:00Z".to_owned(),
                supersedes_seq: None,
            })?;
    }

    for edge in graph.imports() {
        store
            .graph_event_log_mut()
            .append_with_seq(|seq| GraphEventLogEntry {
                schema_version: SCHEMA_VERSION,
                seq,
                id: format!("evt-import-{seq}"),
                event: GraphEventKind::EdgeAdded {
                    from: edge.from_file_id.clone(),
                    to: edge.module_path.clone(),
                    label: "imports".to_owned(),
                },
                ts: "2026-07-07T00:00:00Z".to_owned(),
                supersedes_seq: None,
            })?;
    }

    for edge in graph.calls() {
        store
            .graph_event_log_mut()
            .append_with_seq(|seq| GraphEventLogEntry {
                schema_version: SCHEMA_VERSION,
                seq,
                id: format!("evt-call-{seq}"),
                event: GraphEventKind::EdgeAdded {
                    from: edge.from_file_id.clone(),
                    to: edge.callee.clone(),
                    label: "calls".to_owned(),
                },
                ts: "2026-07-07T00:00:00Z".to_owned(),
                supersedes_seq: None,
            })?;
    }

    for edge in graph.routes() {
        store
            .graph_event_log_mut()
            .append_with_seq(|seq| GraphEventLogEntry {
                schema_version: SCHEMA_VERSION,
                seq,
                id: format!("evt-route-{seq}"),
                event: GraphEventKind::EdgeAdded {
                    from: edge.from_file_id.clone(),
                    to: format!("{} {}", edge.method, edge.path),
                    label: "routes".to_owned(),
                },
                ts: "2026-07-07T00:00:00Z".to_owned(),
                supersedes_seq: None,
            })?;
    }

    drop(store);

    let outcome = read_verified::<GraphEventLogEntry>(&log_path, |entry| entry.seq)?;
    let mut operational = OperationalGraph::open(&sqlite_path)?;
    operational.rebuild(&outcome.entries)?;

    assert_eq!(operational.node_count()?, snapshot.node_count() as u64);
    assert_eq!(operational.edge_count()?, snapshot.edge_count() as u64);
    assert_eq!(operational.nodes_snapshot()?.len(), snapshot.node_count());

    Ok(())
}
