use enforcer_memory::analysis::clustering::detect_clusters;
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

/// Two clearly-separate module groups: `mod_a_1.rs`/`mod_a_2.rs`
/// call each other densely; `mod_b_1.rs`/`mod_b_2.rs` call each
/// other densely; nothing crosses between the two groups.
fn build_two_module_graph(dir: &Path) -> TestResult<CodeGraph> {
    init_repo(dir)?;
    fs::write(dir.join("mod_a_1.rs"), "fn a1() { a2(); }\n")?;
    fs::write(
        dir.join("mod_a_2.rs"),
        "fn a2() { a1_helper(); }\nfn a1_helper() {}\n",
    )?;
    fs::write(dir.join("mod_b_1.rs"), "fn b1() { b2(); }\n")?;
    fs::write(
        dir.join("mod_b_2.rs"),
        "fn b2() { b1_helper(); }\nfn b1_helper() {}\n",
    )?;
    commit_all(dir, "first")?;

    let mut graph = CodeGraph::new();
    let files = vec![
        dir.join("mod_a_1.rs"),
        dir.join("mod_a_2.rs"),
        dir.join("mod_b_1.rs"),
        dir.join("mod_b_2.rs"),
    ];
    graph.index_repository(dir, &files, &Manifest::default())?;
    Ok(graph)
}

// --- hard test: determinism -----------------------------------

#[test]
fn clustering_is_deterministic_across_two_runs() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_two_module_graph(dir.path())?;

    let run1 = detect_clusters(&graph, 20);
    let run2 = detect_clusters(&graph, 20);
    assert_eq!(
        run1, run2,
        "clustering must be deterministic given the same input graph"
    );
    Ok(())
}

// --- hard test: known-fixture cluster membership ----------------

#[test]
fn two_separate_module_groups_land_in_different_clusters() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_two_module_graph(dir.path())?;
    let result = detect_clusters(&graph, 20);

    assert!(
        result.clusters.len() >= 2,
        "expected at least 2 clusters for two disconnected module groups, got {:?}",
        result.clusters
    );

    let file_a1 = "file:mod_a_1.rs";
    let file_a2 = "file:mod_a_2.rs";
    let file_b1 = "file:mod_b_1.rs";
    let file_b2 = "file:mod_b_2.rs";

    let cluster_of = |node_id: &str| -> Option<&str> {
        result
            .clusters
            .iter()
            .find(|c| c.member_node_ids.iter().any(|m| m == node_id))
            .map(|c| c.id.as_str())
    };

    let cluster_a1 = cluster_of(file_a1).ok_or("mod_a_1.rs missing from any cluster")?;
    let cluster_a2 = cluster_of(file_a2).ok_or("mod_a_2.rs missing from any cluster")?;
    let cluster_b1 = cluster_of(file_b1).ok_or("mod_b_1.rs missing from any cluster")?;
    let cluster_b2 = cluster_of(file_b2).ok_or("mod_b_2.rs missing from any cluster")?;

    assert_eq!(
        cluster_a1, cluster_a2,
        "mod_a_1.rs and mod_a_2.rs call each other densely and must land in the same cluster"
    );
    assert_eq!(
        cluster_b1, cluster_b2,
        "mod_b_1.rs and mod_b_2.rs call each other densely and must land in the same cluster"
    );
    assert_ne!(
        cluster_a1, cluster_b1,
        "the two disconnected module groups must land in different clusters"
    );
    Ok(())
}

#[test]
fn empty_graph_produces_empty_clustering_not_panic() {
    let graph = CodeGraph::new();
    let result = detect_clusters(&graph, 20);
    assert!(result.clusters.is_empty());
    assert!(result.inter_cluster_edges.is_empty());
}

#[test]
fn single_isolated_node_becomes_its_own_singleton_cluster() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_repo(dir.path())?;
    fs::write(dir.path().join("solo.rs"), "fn solo() {}\n")?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(
        dir.path(),
        &[dir.path().join("solo.rs")],
        &Manifest::default(),
    )?;

    let result = detect_clusters(&graph, 20);
    // file:solo.rs + sym:solo.rs:...:solo -- two nodes, connected to
    // each other via Contains, disconnected from everything else.
    assert_eq!(result.clusters.len(), 1);
    assert_eq!(result.clusters[0].member_node_ids.len(), 2);
    Ok(())
}

#[test]
fn inter_cluster_edges_count_only_cross_cluster_edges() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_two_module_graph(dir.path())?;
    let result = detect_clusters(&graph, 20);

    for edge in &result.inter_cluster_edges {
        assert_ne!(
            edge.from_cluster, edge.to_cluster,
            "inter_cluster_edges must never report a self-loop cluster edge"
        );
        assert!(edge.count > 0);
    }
    Ok(())
}

#[test]
fn clustering_refinement_uses_bounded_iterator_traversal() {
    let source = include_str!("../src/analysis/clustering.rs");

    assert_eq!(source.matches("for _ in 0..max_iterations").count(), 0);
    assert_eq!(
        source
            .matches("std::iter::repeat_n((), max_iterations)")
            .count(),
        1
    );
}
