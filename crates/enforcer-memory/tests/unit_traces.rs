use enforcer_domain::memory_types::EdgeProvenance;
use enforcer_domain::paths::RepoRoot;
use enforcer_memory::code_graph::{CodeGraph, Manifest};
use enforcer_memory::store::Store;
use enforcer_memory::traces::{
    ingest_trace_records_into_store, replay_trace_records_from_store, TraceRecord,
    TraceRecordStoreBatch, TraceStore,
};
use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::Command;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

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

fn build_fixture_graph(dir: &Path) -> TestResult<CodeGraph> {
    init_repo(dir)?;
    fs::write(dir.join("a.rs"), "fn caller() { helper(); }\n")?;
    fs::write(dir.join("b.rs"), "fn helper() {}\n")?;
    commit_all(dir, "first")?;

    let mut graph = CodeGraph::new();
    let files = vec![dir.join("a.rs"), dir.join("b.rs")];
    graph.index_repository(dir, &files, &Manifest::default())?;
    Ok(graph)
}

#[test]
fn ingest_annotates_an_existing_parsed_edge_with_runtime_count() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;

    // graph.calls() records file:a.rs -> "helper" (callee as
    // written, per code_graph::CallEdge -- see module docs).
    let mut store = TraceStore::new();
    store.ingest(
        &graph,
        &[TraceRecord {
            caller: "file:a.rs".to_string().into(),
            callee: "helper".to_string().into(),
            count: 5.into(),
        }],
    );

    let edges = store.edges(&graph);
    let annotated = edges
        .iter()
        .find(|e| e.caller == "file:a.rs" && e.callee == "helper")
        .ok_or("expected an annotated edge")?;
    assert_eq!(annotated.provenance, EdgeProvenance::Parsed);
    assert_eq!(annotated.observed_count, 5);
    assert!(store.unresolved().is_empty());
    Ok(())
}

#[test]
fn ingest_creates_a_runtime_only_edge_when_no_parsed_edge_exists() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;

    let helper_id = graph
        .symbol_nodes()
        .find(|s| s.name == "helper")
        .map(|s| s.id.clone())
        .ok_or("expected helper symbol")?;
    let caller_id = graph
        .symbol_nodes()
        .find(|s| s.name == "caller")
        .map(|s| s.id.clone())
        .ok_or("expected caller symbol")?;

    let mut store = TraceStore::new();
    // symbol-id -> symbol-id has no matching parsed CallEdge (parsed
    // edges are file_id -> raw callee name), so this must appear as
    // a brand-new Runtime-provenance edge.
    store.ingest(
        &graph,
        &[TraceRecord {
            caller: caller_id.clone().into(),
            callee: helper_id.clone().into(),
            count: 3.into(),
        }],
    );

    let edges = store.edges(&graph);
    let runtime_edge = edges
        .iter()
        .find(|e| e.caller == caller_id && e.callee == helper_id)
        .ok_or("expected a runtime-only edge")?;
    assert_eq!(runtime_edge.provenance, EdgeProvenance::Runtime);
    assert_eq!(runtime_edge.observed_count, 3);
    Ok(())
}

#[test]
fn reingesting_the_same_batch_sums_counts() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;

    let batch = vec![TraceRecord {
        caller: "file:a.rs".to_string().into(),
        callee: "helper".to_string().into(),
        count: 4.into(),
    }];

    let mut store = TraceStore::new();
    store.ingest(&graph, &batch);
    store.ingest(&graph, &batch);

    let edges = store.edges(&graph);
    let edge = edges
        .iter()
        .find(|e| e.caller == "file:a.rs" && e.callee == "helper")
        .ok_or("expected the edge")?;
    assert_eq!(
        edge.observed_count, 8,
        "re-ingesting the same batch twice must SUM counts (documented idempotency choice)"
    );
    Ok(())
}

#[test]
fn reset_then_reingest_replaces_counts() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;

    let batch = vec![TraceRecord {
        caller: "file:a.rs".to_string().into(),
        callee: "helper".to_string().into(),
        count: 10.into(),
    }];

    let mut store = TraceStore::new();
    store.ingest(&graph, &batch);
    store.reset();
    store.ingest(&graph, &batch);

    let edges = store.edges(&graph);
    let edge = edges
        .iter()
        .find(|e| e.caller == "file:a.rs" && e.callee == "helper")
        .ok_or("expected the edge")?;
    assert_eq!(edge.observed_count, 10, "reset() must clear prior counts");
    Ok(())
}

#[test]
fn unknown_caller_or_callee_is_recorded_not_dropped() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;

    let mut store = TraceStore::new();
    store.ingest(
        &graph,
        &[
            TraceRecord {
                caller: "sym:does-not-exist.rs:1:ghost".to_string().into(),
                callee: "helper".to_string().into(),
                count: 1.into(),
            },
            TraceRecord {
                caller: "file:a.rs".to_string().into(),
                callee: "sym:does-not-exist.rs:1:ghost".to_string().into(),
                count: 1.into(),
            },
        ],
    );

    assert_eq!(store.unresolved().len(), 2);
    assert!(store.unresolved()[0].unresolved_caller.is_unresolved());
    assert!(!store.unresolved()[0].unresolved_callee.is_unresolved());
    assert!(!store.unresolved()[1].unresolved_caller.is_unresolved());
    assert!(store.unresolved()[1].unresolved_callee.is_unresolved());

    // Neither malformed record should have been merged into edges().
    let edges = store.edges(&graph);
    assert!(edges
        .iter()
        .all(|e| !e.caller.contains("ghost") && !e.callee.contains("ghost")));
    Ok(())
}

#[test]
fn ingest_is_deterministically_ordered_by_caller_then_callee() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;

    let mut store = TraceStore::new();
    store.ingest(
        &graph,
        &[
            TraceRecord {
                caller: "file:a.rs".to_string().into(),
                callee: "zzz-unknown".to_string().into(),
                count: 1.into(),
            },
            TraceRecord {
                caller: "file:a.rs".to_string().into(),
                callee: "helper".to_string().into(),
                count: 1.into(),
            },
        ],
    );

    let edges_a = store.edges(&graph);
    let edges_b = store.edges(&graph);
    assert_eq!(edges_a, edges_b, "edges() must be deterministic");

    let callers_callees: Vec<(&str, &str)> = edges_a
        .iter()
        .map(|e| (e.caller.as_str(), e.callee.as_str()))
        .collect();
    let mut sorted = callers_callees.clone();
    sorted.sort();
    assert_eq!(
        callers_callees, sorted,
        "edges() must be sorted by (caller, callee)"
    );
    Ok(())
}

#[test]
fn runtime_trace_records_append_to_store_and_replay_projection() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let stores_dir = tempfile::tempdir()?;
    let root: RepoRoot = "C:/Projects/x06-trace-store".parse()?;
    let mut store = Store::init(stores_dir.path(), &root, "2026-07-04T00:00:00Z")?;
    let batch = vec![TraceRecord {
        caller: "file:a.rs".to_string().into(),
        callee: "helper".to_string().into(),
        count: 7.into(),
    }];

    let mut trace_store = TraceStore::new();
    let appended = ingest_trace_records_into_store(
        &mut store,
        &mut trace_store,
        &graph,
        &TraceRecordStoreBatch::new(&batch, "runtime-probe", "2026-07-04T00:00:01Z"),
    )?;
    assert_eq!(appended, 1);
    assert_eq!(store.read_observation_entries()?.entries.len(), 1);

    let mut replayed = TraceStore::new();
    let replayed_count = replay_trace_records_from_store(&store, &mut replayed, &graph)?;
    assert_eq!(replayed_count, 1);
    let edge = replayed
        .edges(&graph)
        .into_iter()
        .find(|edge| edge.caller == "file:a.rs" && edge.callee == "helper")
        .ok_or("expected replayed trace edge")?;
    assert_eq!(edge.observed_count, 7);
    Ok(())
}
