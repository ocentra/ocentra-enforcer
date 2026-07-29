//! Integration coverage for `enforcer_memory::analysis`, moved out of
//! the source module's inline `#[cfg(test)]` block.

use enforcer_domain::memory_types::TraceDirection;
use enforcer_memory::analysis::CodeAdjacency;
use enforcer_memory::code_graph::{CodeGraph, Manifest};
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

/// A tiny multi-file fixture repo: `a.rs` calls `helper` (defined in
/// `b.rs`), `b.rs` imports nothing interesting, `c.rs` is unrelated.
fn build_fixture_graph(dir: &Path) -> TestResult<CodeGraph> {
    init_repo(dir)?;
    fs::write(dir.join("a.rs"), "fn caller() { helper(); }\n")?;
    fs::write(dir.join("b.rs"), "fn helper() {}\n")?;
    fs::write(dir.join("c.rs"), "fn unrelated() {}\n")?;
    commit_all(dir, "first")?;

    let mut graph = CodeGraph::new();
    let files = vec![dir.join("a.rs"), dir.join("b.rs"), dir.join("c.rs")];
    graph.index_repository(dir, &files, &Manifest::default())?;
    Ok(graph)
}

#[test]
fn related_walk_finds_connected_tests_within_depth() -> TestResult<()> {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let file_a = "file:a.rs";
    assert!(
        adjacency.contains_node(file_a).contains_node(),
        "expected a.rs file node in adjacency"
    );

    let related = adjacency.related(file_a, 3);
    let ids: std::collections::HashSet<&str> = related.iter().map(|r| r.node_id.as_str()).collect();
    assert!(
        ids.iter().any(|id| id.contains("caller")),
        "expected a.rs's own caller symbol reachable via Contains edge, got {ids:?}"
    );
    Ok(())
}

#[test]
fn graph_depth_limit_is_enforced() -> TestResult<()> {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let file_a = "file:a.rs";
    let depth0 = adjacency.related(file_a, 0);
    assert!(depth0.is_empty(), "depth 0 must return no related nodes");

    let depth1 = adjacency.related(file_a, 1);
    for node in &depth1 {
        assert!(node.depth <= 1, "node {:?} exceeded requested depth", node);
    }
    Ok(())
}

#[test]
fn upstream_callers_are_found_via_reverse_dependents() -> TestResult<()> {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let helper_id = graph
        .symbol_nodes()
        .find(|s| s.name == "helper")
        .map(|s| s.id.clone())
        .ok_or("expected a helper symbol node")?;

    let upstream = adjacency.reverse_dependents(&helper_id, 3);
    assert!(
        upstream.iter().any(|id| id == "file:a.rs"),
        "expected file:a.rs (the caller) among upstream dependents of helper, got {upstream:?}"
    );
    Ok(())
}

#[test]
fn hotspots_rank_by_total_degree_descending() -> TestResult<()> {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let scores = adjacency.hotspots(5);
    assert_eq!(scores.len(), 5, "hotspot limit should be applied exactly");
    for i in 1..scores.len() {
        assert!(scores[i - 1].total_degree() >= scores[i].total_degree());
    }
    Ok(())
}

#[test]
fn hotspots_with_zero_limit_return_no_nodes() -> TestResult<()> {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    assert!(adjacency.hotspots(0).is_empty());
    Ok(())
}

#[test]
fn unknown_start_node_returns_empty_not_panic() -> TestResult<()> {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    assert!(adjacency.related("file:does-not-exist.rs", 5).is_empty());
    assert!(adjacency
        .trace_calls("file:does-not-exist.rs", TraceDirection::Out, 5)
        .is_empty());
    Ok(())
}
