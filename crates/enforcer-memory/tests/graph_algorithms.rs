//! Integration coverage for X06.3 -- graph algorithms over
//! [`enforcer_memory::code_graph::CodeGraph`]: related-node walk,
//! call-path tracing, reverse dependency traversal, diff impact
//! analysis, architecture overview, ADR-to-node linkage, and the
//! read-only Cypher-subset query DSL (D-05).
//!
//! Fixture repo (`tests/fixtures/memory/graph_algorithms/`):
//!
//! - `widgets.rs`: `list_widgets` calls `load_from_disk`, which calls
//!   `validate` -- a 2-hop call chain inside one file, for depth-limit
//!   and call-path-tracing coverage.
//! - `router.ts`: imports `./widgets` and declares a `GET /widgets`
//!   route -- for import-edge and route-extraction coverage across
//!   languages.
//! - `unrelated.rs`: a standalone function with no edges to the other
//!   two files -- for "not everything is connected" negative coverage.

use enforcer_domain::memory_types::RiskLevel;
use enforcer_domain::memory_types::TraceDirection;
use enforcer_memory::adr::{AdrError, AdrRecord, AdrStore};
use enforcer_memory::analysis::query::{self, QueryError};
use enforcer_memory::analysis::CodeAdjacency;
use enforcer_memory::architecture;
use enforcer_memory::code_graph::{CodeGraph, Manifest};
use enforcer_memory::impact;
use serde_json::json;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/graph_algorithms";

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

/// Copy every fixture file into `dest`, returning the copied paths (for
/// `index_repository`'s `walk_files` argument) -- same pattern as
/// `tests/code_graph_indexer.rs`.
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

fn build_fixture_graph(dir: &Path) -> TestResult<CodeGraph> {
    init_repo(dir)?;
    let files = copy_fixtures(dir)?;
    commit_all(dir, "initial fixture import")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(dir, &files, &Manifest::default())?;
    Ok(graph)
}

fn find_symbol_id(graph: &CodeGraph, name: &str) -> Option<String> {
    graph
        .symbol_nodes()
        .find(|s| s.name == name)
        .map(|s| s.id.clone())
}

// --- hard test: connected tests lookup ------------------------------

#[test]
fn related_walk_finds_connected_symbols_within_depth() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let related = adjacency.related("file:widgets.rs", 3);
    let ids: Vec<&str> = related.iter().map(|r| r.node_id.as_str()).collect();
    assert!(
        ids.iter().any(|id| id.contains("list_widgets")),
        "expected widgets.rs's own list_widgets symbol reachable via Contains edge, got {ids:?}"
    );
    Ok(())
}

#[test]
fn related_walk_does_not_cross_into_unrelated_file() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let related = adjacency.related("file:widgets.rs", 5);
    let ids: Vec<&str> = related.iter().map(|r| r.node_id.as_str()).collect();
    assert!(
        !ids.iter().any(|id| id.contains("standalone")),
        "unrelated.rs's standalone() must not be reachable from widgets.rs, got {ids:?}"
    );
    Ok(())
}

// --- hard test: graph depth limit -----------------------------------

#[test]
fn graph_depth_limit_is_enforced() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let depth0 = adjacency.related("file:widgets.rs", 0);
    assert!(depth0.is_empty(), "depth 0 must return no related nodes");

    let depth1 = adjacency.related("file:widgets.rs", 1);
    assert!(
        depth1.iter().all(|n| n.depth <= 1),
        "no node should exceed the requested max_depth of 1: {depth1:?}"
    );

    // A longer walk from the same start must find at least as many
    // nodes as the depth-1 walk -- router.ts is only reachable from
    // widgets.rs by first crossing the Imports edge to router.ts at
    // depth 1, then Contains-ing into router.ts's own symbols at
    // depth 2, so depth=1 must be a strict subset of depth=5's reach.
    let depth5 = adjacency.related("file:widgets.rs", 5);
    assert!(
        depth5.len() > depth1.len(),
        "expected depth=5 to reach strictly more nodes than depth=1: depth1={depth1:?} depth5={depth5:?}"
    );
    Ok(())
}

// --- hard test: upstream callers (call-path tracing + reverse deps) -
//
// `code_graph`'s `CallEdge` records only the *file* a call was written
// in (`from_file_id`), not the enclosing symbol -- so every call edge
// in the adjacency is `file:<path> -> callee-symbol`, never
// `symbol -> symbol`. `widgets.rs`'s `load_from_disk` calling
// `validate` therefore shows up as `file:widgets.rs` calling both
// `load_from_disk` and `validate` directly (both depth-1 from the
// file), not as a 2-hop symbol chain. These tests exercise the real
// edge shape rather than a call chain the graph model cannot express.

#[test]
fn call_path_tracing_follows_file_to_symbol_call_edges() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let paths = adjacency.trace_calls("file:widgets.rs", TraceDirection::Out, 3);
    let reaches_validate = paths
        .iter()
        .any(|path| path.iter().any(|hop| hop.node_id.contains("validate")));
    let reaches_load_from_disk = paths.iter().any(|path| {
        path.iter()
            .any(|hop| hop.node_id.contains("load_from_disk"))
    });
    assert!(
        reaches_validate && reaches_load_from_disk,
        "expected widgets.rs's Calls edges to reach both load_from_disk and validate, got {paths:?}"
    );
    Ok(())
}

#[test]
fn upstream_callers_are_found_via_reverse_dependents() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let validate_id = find_symbol_id(&graph, "validate").ok_or("expected a validate symbol")?;

    let upstream = adjacency.reverse_dependents(&validate_id, 3);
    assert!(
        upstream.iter().any(|id| id == "file:widgets.rs"),
        "expected file:widgets.rs (the file validate's Calls edge originates from) among upstream callers of validate, got {upstream:?}"
    );
    Ok(())
}

#[test]
fn unknown_node_returns_empty_not_panic() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    assert!(adjacency.related("file:does-not-exist.rs", 5).is_empty());
    assert!(adjacency
        .trace_calls("file:does-not-exist.rs", TraceDirection::Out, 5)
        .is_empty());
    assert!(adjacency
        .reverse_dependents("file:does-not-exist.rs", 5)
        .is_empty());
    Ok(())
}

// --- hard test: crate map (architecture overview) -------------------

#[test]
fn architecture_sections_group_fixture_files_by_directory() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;

    let overview = architecture::build_overview(&graph, 10);
    assert_eq!(overview.total_files_json(), json!(3));
    assert_ne!(overview.total_symbols_json(), json!(0));
    // All 3 fixture files land at the repo root -- a single "." section.
    assert!(overview.sections().iter().any(|s| s.file_count == 3));
    assert!(
        !overview.hotspots().is_empty(),
        "expected at least one hotspot entry over a connected fixture graph"
    );
    let language_counts = overview.language_counts_json();
    let rust_count = language_counts.as_array().and_then(|counts| {
        counts.iter().find_map(|entry| {
            let values = entry.as_array()?;
            (values.first()?.as_str() == Some("Rust")).then(|| values.get(1)?.as_u64())?
        })
    });
    assert_eq!(rust_count, Some(2), "widgets.rs + unrelated.rs are Rust");
    Ok(())
}

#[test]
fn empty_graph_architecture_overview_is_empty_not_panic() {
    let graph = CodeGraph::new();
    let overview = architecture::build_overview(&graph, 10);
    assert!(overview.sections().is_empty());
    assert_eq!(overview.total_files_json(), json!(0));
}

// --- hard test: diff impact ------------------------------------------

#[test]
fn diff_impact_finds_transitively_affected_files_and_symbols() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;

    let report = impact::analyze_diff_impact(&graph, &["widgets.rs".into()], 3.into());
    assert_eq!(report.impacted.len(), 1);
    let impacted = &report.impacted[0];
    assert_eq!(impacted.rel_path, "widgets.rs");
    assert!(
        impacted
            .affected_node_ids
            .iter()
            .any(|id| id.contains("router") || id == "file:router.ts"),
        "expected router.ts (which imports widgets.rs) among impacted nodes, got {:?}",
        impacted.affected_node_ids
    );
    Ok(())
}

#[test]
fn diff_impact_on_unrelated_file_has_low_risk_and_no_cross_file_impact() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;

    let report = impact::analyze_diff_impact(&graph, &["unrelated.rs".into()], 3.into());
    assert_eq!(report.impacted.len(), 1);
    let impacted = &report.impacted[0];
    assert_eq!(impacted.risk, RiskLevel::Low);
    assert!(
        !impacted
            .affected_node_ids
            .iter()
            .any(|id| id.contains("widgets") || id.contains("router")),
        "unrelated.rs must not blast-radius into widgets.rs/router.ts, got {:?}",
        impacted.affected_node_ids
    );
    Ok(())
}

// --- hard test: ADR roundtrip (linked to graph nodes) ----------------

#[test]
fn adr_roundtrip_links_to_a_real_graph_node() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;

    let widgets_file_id = graph
        .file_nodes()
        .find(|f| f.rel_path == "widgets.rs")
        .map(|f| f.id.clone())
        .ok_or("expected a widgets.rs file node")?;

    let mut store = AdrStore::new();
    store.create(
        AdrRecord::new(
            "adr-graphalgs-001",
            "Why widgets.rs loads from disk synchronously",
        )
        .with_section("context", "small fixture repo, no async runtime")
        .with_section("decision", "keep it synchronous")
        .with_linked_node(widgets_file_id.clone()),
    )?;

    let fetched = store.get("adr-graphalgs-001")?;
    assert_eq!(fetched.sections["decision"], "keep it synchronous");

    let linked = store.adrs_for_node(&widgets_file_id);
    assert_eq!(linked.len(), 1);
    assert_eq!(linked[0].id, "adr-graphalgs-001");

    store.update_section("adr-graphalgs-001", "consequences", "no async complexity")?;
    let updated = store.get("adr-graphalgs-001")?;
    assert_eq!(updated.sections.len(), 3);
    Ok(())
}

#[test]
fn adr_lookup_for_unlinked_node_is_empty() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let store = AdrStore::new();

    let unrelated_id = graph
        .file_nodes()
        .find(|f| f.rel_path == "unrelated.rs")
        .map(|f| f.id.clone())
        .ok_or("expected an unrelated.rs file node")?;
    assert!(store.adrs_for_node(&unrelated_id).is_empty());
    Ok(())
}

#[test]
fn adr_get_unknown_id_is_not_found() {
    let store = AdrStore::new();
    let result = store.get("adr-missing");
    assert!(matches!(result, Err(AdrError::NotFound(id)) if id == "adr-missing"));
}

// --- hard test: unsafe query rejection (D-05) -------------------------

#[test]
fn unsafe_write_queries_are_rejected_for_every_write_verb() {
    for verb in ["CREATE", "DELETE", "SET", "MERGE"] {
        let query_text = format!("{verb} (n:Function) RETURN n");
        let result = query::parse(&query_text);
        assert!(
            matches!(result, Err(QueryError::WriteVerbRejected { .. })),
            "expected {verb} to be rejected as a write verb, got {result:?}"
        );
    }
}

#[test]
fn safe_read_query_executes_against_the_fixture_graph() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let parsed = query::parse("MATCH (n:Function) WHERE n.name = 'validate' RETURN n")?;
    let rows = query::execute(&parsed, &adjacency, &graph)?;
    assert_eq!(rows.len(), 1);
    assert!(rows[0]["n"].contains("validate"));
    Ok(())
}

#[test]
fn query_with_relationship_hop_traverses_route_containment() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let parsed = query::parse(
        "MATCH (n:File)-[:CONTAINS*1..2]->(m:Function) RETURN n, m ORDER BY m LIMIT 10",
    )?;
    let rows = query::execute(&parsed, &adjacency, &graph)?;
    assert!(
        !rows.is_empty(),
        "expected at least one File-CONTAINS->Function row over the fixture graph"
    );
    Ok(())
}

#[test]
fn malformed_query_is_a_parse_error_not_a_panic() {
    let result = query::parse("MATCH n RETURN");
    assert!(result.is_err());
    assert!(matches!(result, Err(QueryError::Parse { .. })));
}
