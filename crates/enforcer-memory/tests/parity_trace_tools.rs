//! Integration coverage for X06.P2 -- the `trace_path` parity modes
//! (calls/data_flow/cross_service) over
//! [`enforcer_memory::analysis::trace`] and the `ingest_traces` runtime
//! call-trace merge ([`enforcer_memory::traces`]), plus the
//! [`enforcer_memory::impact`] risk-classification extension.
//!
//! Fixture repo (`tests/fixtures/memory/parity_trace_tools/`):
//!
//! - `service.rs`: `handler` calls `process`, which calls `persist` --
//!   a 3-hop chain (file-to-symbol Calls edges) for `calls`/`data_flow`
//!   depth and 3-hop coverage.
//! - `router.ts`: imports `./service` and declares `POST /items` --
//!   the `cross_service` producer.
//! - `client.ts`: imports `./router` -- a genuine upstream consumer of
//!   the route producer, for `TraceDirection::In` coverage.
//! - `service_test.rs`: a test file calling `handler` -- for
//!   `include_tests` filtering and test-coverage risk-signal coverage.

use enforcer_memory::analysis::trace::{
    self, hop_to_risk_label, Approximation, RiskLabel, TraceCallsParams,
};
use enforcer_memory::analysis::{CodeAdjacency, EdgeKind, TraceDirection};
use enforcer_memory::code_graph::{CodeGraph, CodeNode, Manifest};
use enforcer_memory::impact::{self, ImpactScope, RiskFactors, RiskLevel};
use enforcer_memory::traces::{EdgeProvenance, TraceRecord, TraceStore};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/parity_trace_tools";

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

fn build_fixture_graph(dir: &Path) -> TestResult<CodeGraph> {
    init_repo(dir)?;
    let files = copy_fixtures(dir)?;
    commit_all(dir, "initial fixture import")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(dir, &files, &Manifest::default())?;
    Ok(graph)
}

// --- hard test: calls-mode consistency with X06.3's own traversal ---

#[test]
fn calls_mode_hop_set_matches_underlying_adjacency_trace() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let params = TraceCallsParams {
        direction: TraceDirection::Out,
        depth: 3,
        ..Default::default()
    };
    let report = trace::trace_calls(&adjacency, &graph, "file:service.rs", &params);
    let wrapped_ids = trace::distinct_node_ids(&report);

    let raw = adjacency.trace_calls("file:service.rs", TraceDirection::Out, 3);
    let mut raw_ids: Vec<String> = raw
        .into_iter()
        .flat_map(|p| p.into_iter().map(|h| h.node_id))
        .collect();
    raw_ids.sort();
    raw_ids.dedup();

    assert_eq!(wrapped_ids, raw_ids);
    Ok(())
}

// --- regression: symbol-id start must bridge to its containing file's
// Calls edges (x06-parity tool-diffs.ndjson row 13, "trace_path(calls)":
// "candidate trace_calls did not find load_widget_settings as an
// outbound callee") ---
//
// The baseline resolves `trace_path`'s root to a *symbol* (its
// `function_name` param, per the x06 baseline tool-schemas doc §5.1:
// "resolution: exact `name=` match ... both project-scoped"), then BFS-
// fans-out from that symbol id. But this crate's `CallEdge`/`ImportEdge`
// are recorded at *file* granularity only (`from_file_id`, never an
// enclosing-symbol id) -- so before this fix, `CodeAdjacency::trace_calls`
// starting from a *symbol* node id (rather than a `file:` id, as every
// other test in this suite uses) would immediately dead-end: the symbol
// node's only edge in the adjacency graph is the *incoming* `Contains`
// edge from its file, so an `Out`-direction walk found zero outgoing
// edges and returned no paths at all, silently missing every real
// outbound callee. This pins the fix: tracing `Out` from a symbol id now
// transparently bridges to that symbol's containing file so the file's
// real `Calls` edges are reachable.
#[test]
fn calls_mode_from_a_symbol_start_finds_the_containing_files_outbound_callee() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_repo(dir.path())?;
    fs::write(
        dir.path().join("lib.rs"),
        "fn parse_config_file(path: &str) -> String {\n    load_widget_settings(path)\n}\n",
    )?;
    fs::write(
        dir.path().join("widget.rs"),
        "fn load_widget_settings(path: &str) -> String {\n    path.to_string()\n}\n",
    )?;
    commit_all(dir.path(), "x06-parity trace_path(calls) fixture")?;

    let files = vec![dir.path().join("lib.rs"), dir.path().join("widget.rs")];
    let mut graph = CodeGraph::new();
    graph.index_repository(dir.path(), &files, &Manifest::default())?;
    let adjacency = CodeAdjacency::build(&graph);

    let start_symbol_id = graph
        .nodes()
        .iter()
        .find_map(|node| match node {
            CodeNode::Function(sym) if sym.name == "parse_config_file" => Some(sym.id.clone()),
            _ => None,
        })
        .ok_or("expected a parse_config_file function symbol in the fixture graph")?;

    let report = trace::trace_calls(
        &adjacency,
        &graph,
        &start_symbol_id,
        &TraceCallsParams {
            direction: TraceDirection::Out,
            depth: 3,
            ..Default::default()
        },
    );
    let ids = trace::distinct_node_ids(&report);
    assert!(
        ids.iter().any(|id| id.contains("widget")),
        "expected trace_calls(Out) from parse_config_file's symbol id to reach \
         load_widget_settings via the file-level Calls edge bridge, got {ids:?}"
    );

    // Same bridge, unwrapped: `CodeAdjacency::trace_calls` directly (the
    // exact call `trace::trace_calls` wraps) must also find it, since
    // the bridging lives in `CodeAdjacency::trace_calls` itself.
    let raw_ids: Vec<String> = adjacency
        .trace_calls(&start_symbol_id, TraceDirection::Out, 3)
        .into_iter()
        .flat_map(|p| p.into_iter().map(|h| h.node_id))
        .collect();
    assert!(
        raw_ids.iter().any(|id| id.contains("widget")),
        "expected raw CodeAdjacency::trace_calls to reach load_widget_settings too, got {raw_ids:?}"
    );
    Ok(())
}

// --- hard test: data_flow 3-hop arg->param linkage (honest approximation) ---
//
// `code_graph`'s `CallEdge` records only the *file* a call was written
// in (`from_file_id`), never the enclosing symbol (see
// `code_graph`'s/`analysis`'s own module docs) -- so a real multi-hop
// chain from one seed requires crossing file boundaries via `Imports`
// edges between each hop, not chained `Calls` edges within one file
// (two calls in the same file are both hop-1 from that file, not
// chained). client.ts -> router.ts -> service.rs is exactly that
// 3-hop chain: client.ts imports router.ts (hop 1, Imports), router.ts
// imports service.rs (hop 2, Imports), service.rs calls handler/
// process/persist (hop 3, Calls).

#[test]
fn data_flow_mode_reaches_the_full_three_hop_chain_and_stays_honest() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let params = TraceCallsParams {
        direction: TraceDirection::Out,
        depth: 3,
        ..Default::default()
    };
    let report = trace::trace_data_flow(&adjacency, &graph, "file:client.ts", &params);

    assert_eq!(report.approximation, Approximation::CallGraphOnly);
    let reaches_router = report
        .paths
        .iter()
        .any(|p| p.hops.iter().any(|h| h.hop.node_id == "file:router.ts"));
    let reaches_service = report
        .paths
        .iter()
        .any(|p| p.hops.iter().any(|h| h.hop.node_id == "file:service.rs"));
    let reaches_handler = report
        .paths
        .iter()
        .any(|p| p.hops.iter().any(|h| h.hop.node_id.contains("handler")));
    assert!(
        reaches_router && reaches_service && reaches_handler,
        "expected the 3-hop client.ts->router.ts->service.rs->handler chain reachable via \
         data_flow within depth 3, got {:?}",
        report.paths
    );
    // At least one path must actually realize all 3 hops in sequence,
    // not just reach each node via three separate, shorter paths.
    let has_full_three_hop_path = report.paths.iter().any(|p| {
        p.hops.len() >= 3
            && p.hops[0].hop.node_id == "file:router.ts"
            && p.hops[1].hop.node_id == "file:service.rs"
            && p.hops[2].hop.node_id.contains("handler")
    });
    assert!(
        has_full_three_hop_path,
        "expected a single path realizing all 3 hops in order, got {:?}",
        report.paths
    );
    for path in &report.paths {
        for hop in &path.hops {
            assert!(
                hop.param_link.is_none(),
                "no arg->param linkage data exists in this crate's parser layer -- \
                 data_flow must never fabricate one"
            );
        }
    }
    Ok(())
}

// --- hard test: cross_service producer -> route -> consumer ---------

#[test]
fn cross_service_mode_finds_producer_route_and_upstream_consumer() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    // router.ts declares POST /items; client.ts imports router.ts, so
    // client.ts is a genuine upstream consumer of that route.
    let report = trace::trace_cross_service(
        &adjacency,
        &graph,
        "file:client.ts",
        trace::TraceCrossServiceParams {
            direction: TraceDirection::In,
            depth: 3,
            include_tests: true,
        },
    );
    assert!(
        report.paths.iter().any(|p| p.mediator.method == "POST"
            && p.mediator.path == "/items"
            && p.mediator.producer_node_id == "file:router.ts"
            && p.consumer_node_id == "file:client.ts"),
        "expected client.ts reported as a consumer of router.ts's POST /items route, got {:?}",
        report.paths
    );
    Ok(())
}

// --- hard test: direction/depth semantics ----------------------------

#[test]
fn direction_and_depth_bound_the_calls_mode_hop_set() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let shallow = trace::trace_calls(
        &adjacency,
        &graph,
        "file:service.rs",
        &TraceCallsParams {
            direction: TraceDirection::Out,
            depth: 1,
            ..Default::default()
        },
    );
    for path in &shallow.paths {
        assert!(path.hops.len() <= 1, "depth=1 must not exceed 1 hop");
    }

    let deep = trace::trace_calls(
        &adjacency,
        &graph,
        "file:service.rs",
        &TraceCallsParams {
            direction: TraceDirection::Out,
            depth: 3,
            ..Default::default()
        },
    );
    assert!(
        trace::distinct_node_ids(&deep).len() >= trace::distinct_node_ids(&shallow).len(),
        "a deeper trace must reach at least as many nodes as a shallower one"
    );
    Ok(())
}

// --- hard test: include_tests filter, file-level test hops included --

#[test]
fn include_tests_false_excludes_the_test_files_own_hop() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let handler_id = graph
        .symbol_nodes()
        .find(|s| s.name == "handler")
        .map(|s| s.id.clone())
        .ok_or("expected handler symbol")?;

    let with_tests = trace::trace_calls(
        &adjacency,
        &graph,
        &handler_id,
        &TraceCallsParams {
            direction: TraceDirection::In,
            depth: 3,
            include_tests: true,
            ..Default::default()
        },
    );
    let without_tests = trace::trace_calls(
        &adjacency,
        &graph,
        &handler_id,
        &TraceCallsParams {
            direction: TraceDirection::In,
            depth: 3,
            include_tests: false,
            ..Default::default()
        },
    );

    let with_ids = trace::distinct_node_ids(&with_tests);
    let without_ids = trace::distinct_node_ids(&without_tests);
    assert!(
        with_ids.iter().any(|id| id.contains("service_test")),
        "expected service_test.rs reachable when include_tests=true, got {with_ids:?}"
    );
    assert!(
        !without_ids.iter().any(|id| id.contains("service_test")),
        "expected service_test.rs excluded (file-level Calls hop included) when \
         include_tests=false, got {without_ids:?}"
    );
    Ok(())
}

// --- hard test: edge_types filter -------------------------------------

#[test]
fn edge_types_filter_restricts_every_hop_kind() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let calls_only = trace::trace_calls(
        &adjacency,
        &graph,
        "file:service.rs",
        &TraceCallsParams {
            direction: TraceDirection::Out,
            depth: 3,
            edge_types: Some(&[EdgeKind::Calls]),
            ..Default::default()
        },
    );
    for path in &calls_only.paths {
        for hop in &path.hops {
            assert_eq!(hop.via, EdgeKind::Calls);
        }
    }
    Ok(())
}

// --- hard test: parity risk_labels (hop-distance, baseline-verified) -

#[test]
fn risk_labels_follow_the_baseline_hop_distance_scheme() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    assert_eq!(hop_to_risk_label(1), RiskLabel::Critical);
    assert_eq!(hop_to_risk_label(2), RiskLabel::High);
    assert_eq!(hop_to_risk_label(3), RiskLabel::Medium);
    assert_eq!(hop_to_risk_label(4), RiskLabel::Low);
    assert_eq!(hop_to_risk_label(100), RiskLabel::Low);
    assert_eq!(RiskLabel::Critical.as_str(), "CRITICAL");
    assert_eq!(RiskLabel::Low.as_str(), "LOW");

    let without_labels = trace::trace_calls(
        &adjacency,
        &graph,
        "file:service.rs",
        &TraceCallsParams::default(),
    );
    for path in &without_labels.paths {
        assert!(
            path.risk_labels.is_none(),
            "risk_labels defaults to false -- must be None, never an empty Vec masquerading \
             as 'no labels asked for'"
        );
    }

    let with_labels = trace::trace_calls(
        &adjacency,
        &graph,
        "file:service.rs",
        &TraceCallsParams {
            direction: TraceDirection::Out,
            depth: 3,
            risk_labels: true,
            ..Default::default()
        },
    );
    for path in &with_labels.paths {
        let labels = path
            .risk_labels
            .as_ref()
            .ok_or("expected risk_labels populated when risk_labels=true")?;
        assert_eq!(
            labels.len(),
            path.hops.len(),
            "risk_labels must be a parallel array, one entry per hop"
        );
        for (i, label) in labels.iter().enumerate() {
            assert_eq!(*label, hop_to_risk_label(i + 1));
        }
    }
    Ok(())
}

// --- hard test: ingest_traces merge, idempotency, unresolved capture -

#[test]
fn ingest_traces_annotates_parsed_edges_with_runtime_counts() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;

    let mut store = TraceStore::new();
    store.ingest(
        &graph,
        &[TraceRecord {
            caller: "file:service.rs".to_string(),
            callee: "process".to_string(),
            count: 7,
        }],
    );

    let edges = store.edges(&graph);
    let annotated = edges
        .iter()
        .find(|e| e.caller == "file:service.rs" && e.callee == "process")
        .ok_or("expected an annotated parsed edge")?;
    assert_eq!(annotated.provenance, EdgeProvenance::Parsed);
    assert_eq!(annotated.observed_count, 7);
    assert!(store.unresolved().is_empty());
    Ok(())
}

#[test]
fn ingest_traces_creates_runtime_only_edges_for_resolved_symbol_pairs() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;

    let handler_id = graph
        .symbol_nodes()
        .find(|s| s.name == "handler")
        .map(|s| s.id.clone())
        .ok_or("expected handler symbol")?;
    let process_id = graph
        .symbol_nodes()
        .find(|s| s.name == "process")
        .map(|s| s.id.clone())
        .ok_or("expected process symbol")?;

    let mut store = TraceStore::new();
    store.ingest(
        &graph,
        &[TraceRecord {
            caller: handler_id.clone(),
            callee: process_id.clone(),
            count: 4,
        }],
    );

    let edges = store.edges(&graph);
    let runtime_edge = edges
        .iter()
        .find(|e| e.caller == handler_id && e.callee == process_id)
        .ok_or("expected a runtime-only edge for the resolved symbol pair")?;
    assert_eq!(runtime_edge.provenance, EdgeProvenance::Runtime);
    assert_eq!(runtime_edge.observed_count, 4);
    Ok(())
}

#[test]
fn ingest_traces_reingestion_sums_counts_idempotently() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;

    let batch = vec![TraceRecord {
        caller: "file:service.rs".to_string(),
        callee: "process".to_string(),
        count: 3,
    }];

    let mut store = TraceStore::new();
    store.ingest(&graph, &batch);
    store.ingest(&graph, &batch);
    store.ingest(&graph, &batch);

    let edges = store.edges(&graph);
    let edge = edges
        .iter()
        .find(|e| e.caller == "file:service.rs" && e.callee == "process")
        .ok_or("expected the edge")?;
    assert_eq!(
        edge.observed_count, 9,
        "re-ingesting the same batch 3 times must SUM counts (documented idempotency choice)"
    );

    store.reset();
    store.ingest(&graph, &batch);
    let edges_after_reset = store.edges(&graph);
    let edge_after_reset = edges_after_reset
        .iter()
        .find(|e| e.caller == "file:service.rs" && e.callee == "process")
        .ok_or("expected the edge after reset")?;
    assert_eq!(
        edge_after_reset.observed_count, 3,
        "reset() must clear prior counts before the next ingest"
    );
    Ok(())
}

#[test]
fn ingest_traces_never_drops_unresolved_records() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;

    let mut store = TraceStore::new();
    store.ingest(
        &graph,
        &[
            TraceRecord {
                caller: "sym:does-not-exist.rs:1:ghost".to_string(),
                callee: "process".to_string(),
                count: 1,
            },
            TraceRecord {
                caller: "file:service.rs".to_string(),
                callee: "sym:does-not-exist.rs:1:ghost".to_string(),
                count: 1,
            },
            TraceRecord {
                caller: "sym:does-not-exist.rs:1:also-ghost".to_string(),
                callee: "sym:does-not-exist.rs:1:ghost-too".to_string(),
                count: 1,
            },
        ],
    );

    assert_eq!(store.unresolved().len(), 3);
    assert!(store.unresolved()[0].unresolved_caller);
    assert!(!store.unresolved()[0].unresolved_callee);
    assert!(!store.unresolved()[1].unresolved_caller);
    assert!(store.unresolved()[1].unresolved_callee);
    assert!(store.unresolved()[2].unresolved_caller);
    assert!(store.unresolved()[2].unresolved_callee);

    let edges = store.edges(&graph);
    assert!(edges
        .iter()
        .all(|e| !e.caller.contains("ghost") && !e.callee.contains("ghost")));
    Ok(())
}

// --- hard test: risk boundaries (high-centrality vs leaf, tested vs untested) --

#[test]
fn risk_boundaries_high_centrality_beats_test_coverage() {
    let high_centrality_untested = RiskFactors {
        centrality_degree: 25,
        has_test_coverage: false,
        has_downstream_route: false,
    };
    let high_centrality_tested = RiskFactors {
        centrality_degree: 25,
        has_test_coverage: true,
        has_downstream_route: false,
    };
    assert_eq!(
        impact::classify_risk_from_factors(high_centrality_untested),
        RiskLevel::High
    );
    assert_eq!(
        impact::classify_risk_from_factors(high_centrality_tested),
        RiskLevel::High,
        "a highly-connected node stays High risk even when tested -- tests reduce risk of \
         regressions going unnoticed, not the blast radius itself"
    );
}

#[test]
fn risk_boundaries_leaf_node_tested_is_low() {
    let leaf = RiskFactors {
        centrality_degree: 0,
        has_test_coverage: true,
        has_downstream_route: false,
    };
    assert_eq!(impact::classify_risk_from_factors(leaf), RiskLevel::Low);
}

#[test]
fn risk_boundaries_untested_downstream_route_is_high() {
    let untested_route = RiskFactors {
        centrality_degree: 1,
        has_test_coverage: false,
        has_downstream_route: true,
    };
    assert_eq!(
        impact::classify_risk_from_factors(untested_route),
        RiskLevel::High
    );
}

#[test]
fn scoped_impact_over_the_fixture_graph_finds_the_downstream_route_and_test_coverage() -> TestResult
{
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;

    // Changing service.rs must find router.ts's POST /items downstream
    // (router.ts imports service.rs) AND test coverage (service_test.rs
    // calls handler, a file-level Calls hop -- exercising the same
    // test_node_ids file-id fix `include_tests` relies on).
    let report = impact::analyze_diff_impact_scoped(
        &graph,
        &["service.rs".to_string()],
        impact::DEFAULT_DEPTH,
        ImpactScope::All,
    );
    assert_eq!(report.impacted.len(), 1);
    let impacted = &report.impacted[0];
    assert!(
        impacted.factors.has_downstream_route,
        "expected router.ts's POST /items downstream of service.rs, got {:?}",
        impacted.factors
    );
    assert!(
        impacted.factors.has_test_coverage,
        "expected service_test.rs's coverage of handler() to count, got {:?}",
        impacted.factors
    );
    Ok(())
}
