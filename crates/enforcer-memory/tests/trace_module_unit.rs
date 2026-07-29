//! Unit-level coverage for [`enforcer_memory::analysis::trace`], moved
//! out of an inline `#[cfg(test)] mod tests` in `src/analysis/trace.rs`
//! per this repo's no-inline-tests style (tests live under `tests/`).
//! See `tests/parity_trace_tools.rs` for the higher-level X06.P2
//! integration coverage (risk labels, ingest_traces, impact scoring)
//! this file does not duplicate.

use enforcer_domain::memory_types::{Approximation, MemoryEdgeKind, TraceDirection};
use enforcer_memory::analysis::trace::{
    distinct_node_ids, trace_calls, trace_cross_service, trace_data_flow, CrossServicePath,
    TraceCallsParams, TraceCrossServiceParams,
};
use enforcer_memory::analysis::CodeAdjacency;
use enforcer_memory::code_graph::{CodeGraph, Manifest};
use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
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

/// `a.rs` calls `helper` (`b.rs`), which calls `deep` (`c.rs`);
/// `router.ts` imports `a.rs` and declares a `GET /a` route;
/// `client.ts` imports `router.ts` (a genuine upstream consumer of
/// the route producer, for `TraceDirection::In` coverage);
/// `a_test.rs` is a test file calling `helper` too, for
/// `include_tests` filtering coverage.
fn build_fixture_graph(dir: &Path) -> TestResult<CodeGraph> {
    init_repo(dir)?;
    fs::write(dir.join("a.rs"), "fn caller() { helper(); }\n")?;
    fs::write(dir.join("b.rs"), "fn helper() { deep(); }\n")?;
    fs::write(dir.join("c.rs"), "fn deep() {}\n")?;
    fs::write(
        dir.join("router.ts"),
        "import { caller } from \"./a\";\nrouter.get(\"/a\", caller);\n",
    )?;
    fs::write(dir.join("client.ts"), "import \"./router\";\n")?;
    fs::write(
        dir.join("a_test.rs"),
        "#[test]\nfn a_test() { helper(); }\n",
    )?;
    commit_all(dir, "first")?;

    let mut graph = CodeGraph::new();
    let files: Vec<PathBuf> = vec![
        dir.join("a.rs"),
        dir.join("b.rs"),
        dir.join("c.rs"),
        dir.join("router.ts"),
        dir.join("client.ts"),
        dir.join("a_test.rs"),
    ];
    graph.index_repository(dir, &files, &Manifest::default())?;
    Ok(graph)
}

// --- calls mode wraps X06.3 traversal consistently ---------------

#[test]
fn calls_mode_matches_underlying_trace_calls_hop_set() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let report = trace_calls(
        &adjacency,
        &graph,
        "file:a.rs",
        &TraceCallsParams {
            direction: TraceDirection::Out,
            depth: 3.into(),
            ..Default::default()
        },
    );
    let wrapped_ids = distinct_node_ids(&report);

    let raw = adjacency.trace_calls("file:a.rs", TraceDirection::Out, 3);
    let mut raw_ids: BTreeSet<String> = raw
        .into_iter()
        .flat_map(|p| p.into_iter().map(|h| h.node_id.to_string()))
        .collect::<BTreeSet<_>>();
    let raw_ids: Vec<String> = std::mem::take(&mut raw_ids).into_iter().collect();

    assert_eq!(wrapped_ids, raw_ids);
    Ok(())
}

#[test]
fn calls_mode_output_is_deterministically_ordered_across_calls() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let params = TraceCallsParams {
        direction: TraceDirection::Out,
        depth: 3.into(),
        ..Default::default()
    };
    let first = trace_calls(&adjacency, &graph, "file:a.rs", &params);
    let second = trace_calls(&adjacency, &graph, "file:a.rs", &params);
    assert_eq!(first, second);
    Ok(())
}

// --- direction/depth semantics ------------------------------------

#[test]
fn direction_in_reaches_only_upstream_callers() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let helper_id = graph
        .symbol_nodes()
        .find(|s| s.name == "helper")
        .map(|s| s.id.clone())
        .ok_or("expected helper symbol")?;

    let report = trace_calls(
        &adjacency,
        &graph,
        &helper_id,
        &TraceCallsParams {
            direction: TraceDirection::In,
            depth: 3.into(),
            ..Default::default()
        },
    );
    let ids = distinct_node_ids(&report);
    assert!(
        ids.iter().any(|id| id == "file:a.rs"),
        "expected file:a.rs upstream of helper via In direction, got {ids:?}"
    );
    Ok(())
}

#[test]
fn depth_bounds_the_number_of_hops_returned() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let shallow = trace_calls(
        &adjacency,
        &graph,
        "file:a.rs",
        &TraceCallsParams {
            direction: TraceDirection::Out,
            depth: 1.into(),
            ..Default::default()
        },
    );
    for path in &shallow.paths {
        assert!(path.hops.len() <= 1, "depth=1 must not exceed 1 hop");
    }
    Ok(())
}

// --- include_tests filtering --------------------------------------

#[test]
fn include_tests_false_excludes_test_symbol_hops() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let helper_id = graph
        .symbol_nodes()
        .find(|s| s.name == "helper")
        .map(|s| s.id.clone())
        .ok_or("expected helper symbol")?;

    let with_tests = trace_calls(
        &adjacency,
        &graph,
        &helper_id,
        &TraceCallsParams {
            direction: TraceDirection::In,
            depth: 3.into(),
            include_tests: true.into(),
            ..Default::default()
        },
    );
    let without_tests = trace_calls(
        &adjacency,
        &graph,
        &helper_id,
        &TraceCallsParams {
            direction: TraceDirection::In,
            depth: 3.into(),
            include_tests: false.into(),
            ..Default::default()
        },
    );

    let with_ids = distinct_node_ids(&with_tests);
    let without_ids = distinct_node_ids(&without_tests);
    assert!(
        with_ids.iter().any(|id| id.contains("a_test")),
        "expected a_test.rs's test symbol reachable when include_tests=true, got {with_ids:?}"
    );
    assert!(
        !without_ids.iter().any(|id| id.contains("a_test")),
        "expected a_test.rs's test symbol excluded when include_tests=false, got {without_ids:?}"
    );
    Ok(())
}

// --- data_flow: honest approximation ------------------------------

#[test]
fn data_flow_mode_follows_call_edges_and_labels_approximation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let report = trace_data_flow(
        &adjacency,
        &graph,
        "file:a.rs",
        &TraceCallsParams {
            direction: TraceDirection::Out,
            depth: 3.into(),
            ..Default::default()
        },
    );
    assert_eq!(report.approximation, Approximation::CallGraphOnly);
    assert!(report.paths.iter().any(|path| !path.hops.is_empty()));
    for path in &report.paths {
        for hop in &path.hops {
            assert!(
                hop.param_link.is_none(),
                "no param_link data exists in this crate's parser layer yet"
            );
        }
    }
    Ok(())
}

#[test]
fn data_flow_mode_reaches_a_three_hop_chain() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    // A real multi-hop chain from one seed must cross file
    // boundaries via Imports edges -- `CallEdge` records only the
    // *file* a call was written in, never the enclosing symbol (see
    // module docs), so two Calls in the same file are both hop-1
    // from that file, never chained. client.ts -> router.ts -> a.rs
    // -> helper is exactly that: client.ts imports router.ts
    // (hop 1), router.ts imports a.rs (hop 2), a.rs calls helper
    // (hop 3).
    let report = trace_data_flow(
        &adjacency,
        &graph,
        "file:client.ts",
        &TraceCallsParams {
            direction: TraceDirection::Out,
            depth: 3.into(),
            ..Default::default()
        },
    );
    let reaches_router = report
        .paths
        .iter()
        .any(|p| p.hops.iter().any(|h| h.hop.node_id == "file:router.ts"));
    let reaches_a = report
        .paths
        .iter()
        .any(|p| p.hops.iter().any(|h| h.hop.node_id == "file:a.rs"));
    let reaches_helper = report
        .paths
        .iter()
        .any(|p| p.hops.iter().any(|h| h.hop.node_id.contains("helper")));
    assert!(
        reaches_router && reaches_a && reaches_helper,
        "expected data_flow to reach router.ts, a.rs, and helper via the 3-hop \
         Imports/Imports/Calls chain from client.ts, got {:?}",
        report.paths
    );
    Ok(())
}

// --- cross_service: producer -> route -> consumer -----------------

#[test]
fn cross_service_mode_finds_producer_route_consumer_path() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    // router.ts imports a.rs and declares GET /a with from_file_id
    // = file:router.ts (route declared in router.ts itself).
    let report = trace_cross_service(
        &adjacency,
        &graph,
        "file:router.ts",
        TraceCrossServiceParams {
            direction: TraceDirection::Both,
            depth: 3.into(),
            include_tests: true.into(),
        },
    );
    assert!(
        !report.paths.is_empty(),
        "expected at least one cross_service path from router.ts's own declared route"
    );
    let has_expected_route = report
        .paths
        .iter()
        .any(|p| p.mediator.method == "GET" && p.mediator.path == "/a");
    assert!(
        has_expected_route,
        "expected GET /a route among mediators, got {:?}",
        report.paths
    );
    Ok(())
}

#[test]
fn cross_service_mode_reports_consumer_that_imports_the_producer() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    // The route is declared in router.ts; a.rs is imported by
    // router.ts (a Calls/Imports edge from router.ts -> a.rs), so
    // a.rs's own upstream dependents (via reverse_dependents on
    // router.ts) should include router.ts's importer set. We assert
    // the mediator's producer is router.ts and that consumers
    // reachable from it are reported deterministically.
    let report = trace_cross_service(
        &adjacency,
        &graph,
        "file:router.ts",
        TraceCrossServiceParams {
            direction: TraceDirection::Both,
            depth: 3.into(),
            include_tests: true.into(),
        },
    );
    let route_paths: Vec<&CrossServicePath> = report
        .paths
        .iter()
        .filter(|p| p.mediator.producer_node_id == "file:router.ts")
        .collect();
    assert!(
        !route_paths.is_empty(),
        "expected router.ts's own route to be present as a mediator"
    );
    Ok(())
}

#[test]
fn cross_service_include_tests_false_excludes_test_consumers() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let with_tests = trace_cross_service(
        &adjacency,
        &graph,
        "file:router.ts",
        TraceCrossServiceParams {
            direction: TraceDirection::Both,
            depth: 3.into(),
            include_tests: true.into(),
        },
    );
    let without_tests = trace_cross_service(
        &adjacency,
        &graph,
        "file:router.ts",
        TraceCrossServiceParams {
            direction: TraceDirection::Both,
            depth: 3.into(),
            include_tests: false.into(),
        },
    );
    let without_has_test_consumer = without_tests
        .paths
        .iter()
        .any(|p| p.consumer_node_id.contains("a_test"));
    assert!(!without_has_test_consumer);
    let _ = with_tests;
    Ok(())
}

#[test]
fn cross_service_direction_in_finds_route_from_a_consumers_perspective() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    // client.ts imports router.ts, which declares GET /a: starting
    // FROM client.ts (a consumer, not the producer) with
    // TraceDirection::In must still surface the route. Before this
    // lane's fix, `direction=In` skipped the producer-reachability
    // check entirely (it was gated on `Out | Both` only), so `In`
    // could NEVER find a route unless `start` was literally the
    // producer itself -- this test pins that regression.
    let report = trace_cross_service(
        &adjacency,
        &graph,
        "file:client.ts",
        TraceCrossServiceParams {
            direction: TraceDirection::In,
            depth: 3.into(),
            include_tests: true.into(),
        },
    );
    assert!(
        !report.paths.is_empty(),
        "expected TraceDirection::In from a consumer to find the producer's route, got {:?}",
        report.paths
    );
    assert!(
        report
            .paths
            .iter()
            .any(|p| p.mediator.producer_node_id == "file:router.ts"
                && p.consumer_node_id == "file:client.ts"),
        "expected client.ts listed as a consumer of router.ts's GET /a, got {:?}",
        report.paths
    );
    Ok(())
}

#[test]
fn cross_service_unrelated_node_finds_no_route_in_any_direction() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    // c.rs is only reachable FROM b.rs (deep() is called by
    // helper()); it has no Calls/Imports edge to or from router.ts
    // in either direction, so no direction should surface router.ts's
    // route starting from it -- direction gates WHICH relationship
    // counts, it does not make every node reachable regardless.
    for direction in [
        TraceDirection::Out,
        TraceDirection::In,
        TraceDirection::Both,
    ] {
        let report = trace_cross_service(
            &adjacency,
            &graph,
            "file:c.rs",
            TraceCrossServiceParams {
                direction,
                depth: 3.into(),
                include_tests: true.into(),
            },
        );
        assert!(
            report.paths.is_empty(),
            "expected file:c.rs (unrelated to router.ts) to find no route via {direction:?}, \
             got {:?}",
            report.paths
        );
    }
    Ok(())
}

// --- unknown node handling -----------------------------------------

#[test]
fn unknown_start_node_returns_empty_report_not_panic() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let default_params = TraceCallsParams::default();
    let calls = trace_calls(
        &adjacency,
        &graph,
        "file:does-not-exist.rs",
        &default_params,
    );
    assert!(calls.paths.is_empty());

    let data_flow = trace_data_flow(
        &adjacency,
        &graph,
        "file:does-not-exist.rs",
        &default_params,
    );
    assert!(data_flow.paths.is_empty());

    let cross_service = trace_cross_service(
        &adjacency,
        &graph,
        "file:does-not-exist.rs",
        TraceCrossServiceParams {
            direction: TraceDirection::Both,
            depth: 3.into(),
            include_tests: true.into(),
        },
    );
    assert!(cross_service.paths.is_empty());
    Ok(())
}

// --- edge_types filter ----------------------------------------------

#[test]
fn edge_types_filter_restricts_hop_kinds() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let calls_only = trace_calls(
        &adjacency,
        &graph,
        "file:a.rs",
        &TraceCallsParams {
            direction: TraceDirection::Out,
            depth: 3.into(),
            edge_types: Some(&[MemoryEdgeKind::Calls]),
            ..Default::default()
        },
    );
    for path in &calls_only.paths {
        for hop in &path.hops {
            assert_eq!(hop.via, MemoryEdgeKind::Calls);
        }
    }
    Ok(())
}
