use enforcer_memory::code_graph::{CodeGraph, Manifest};
use enforcer_memory::traces::{EdgeProvenance, TraceRecord, TraceStore};
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
            caller: "file:a.rs".to_string(),
            callee: "helper".to_string(),
            count: 5,
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
            caller: caller_id.clone(),
            callee: helper_id.clone(),
            count: 3,
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
        caller: "file:a.rs".to_string(),
        callee: "helper".to_string(),
        count: 4,
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
        caller: "file:a.rs".to_string(),
        callee: "helper".to_string(),
        count: 10,
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
                caller: "sym:does-not-exist.rs:1:ghost".to_string(),
                callee: "helper".to_string(),
                count: 1,
            },
            TraceRecord {
                caller: "file:a.rs".to_string(),
                callee: "sym:does-not-exist.rs:1:ghost".to_string(),
                count: 1,
            },
        ],
    );

    assert_eq!(store.unresolved().len(), 2);
    assert!(store.unresolved()[0].unresolved_caller);
    assert!(!store.unresolved()[0].unresolved_callee);
    assert!(!store.unresolved()[1].unresolved_caller);
    assert!(store.unresolved()[1].unresolved_callee);

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
                caller: "file:a.rs".to_string(),
                callee: "zzz-unknown".to_string(),
                count: 1,
            },
            TraceRecord {
                caller: "file:a.rs".to_string(),
                callee: "helper".to_string(),
                count: 1,
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
