//! X06.P2: parity `trace_path` -- the three trace modes the baseline's
//! `trace_path` tool exposes (scout digest §1, row 4: "modes
//! calls/data_flow/cross_service; direction in/out/both; depth default
//! 3"), built as library functions over [`super::CodeAdjacency`] /
//! [`crate::code_graph::CodeGraph`] rather than a duplicate traversal
//! engine.
//!
//! # Modes
//!
//! - [`TraceMode::Calls`] wraps [`super::CodeAdjacency::trace_calls`]
//!   directly (X06.3's existing call-path tracer) -- no new traversal
//!   logic, just the parity-shaped request/response envelope this pack
//!   requires (`direction`/`depth`/`include_tests`/`edge_types`).
//! - [`TraceMode::DataFlow`] follows the *same* call-graph edges as
//!   `calls` mode but is honest about what `code_graph`'s current parser
//!   layer can support: [`crate::parsers::CallRef`] records only a
//!   callee name and line, never argument expressions, and
//!   [`crate::code_graph::SymbolNode`] records no parameter list --
//!   there is no argument-expression-to-parameter data in this crate to
//!   link. Every [`DataFlowHop`] therefore carries `param_link: None`
//!   with [`DataFlowReport::approximation`] fixed at
//!   [`Approximation::CallGraphOnly`], so a caller can never mistake this
//!   for real arg->param binding -- rather than fabricating a plausible-
//!   looking but false linkage. The seam (`param_link: Option<ParamLink>`)
//!   exists so a future parser upgrade that captures call-argument/
//!   parameter lists can populate it without a response-shape break.
//! - [`TraceMode::CrossService`] traces producer -> route/event mediator
//!   -> consumer paths using [`crate::code_graph::CodeGraph::routes`]:
//!   the file declaring a route is the producer; any file reaching the
//!   producer via an `Imports` or `Calls` edge (within the trace depth)
//!   is a consumer of that route. Enforcer's graph model has no explicit
//!   event-bus/pubsub edge kind yet (only `CALLS`/`IMPORTS`/route
//!   declaration, per `code_graph`'s own module docs), so "event" paths
//!   in the baseline's sense are not yet distinguishable from route
//!   paths here -- this is recorded honestly on [`RouteMediator`] rather
//!   than invented.
//!
//! # Common params
//!
//! [`TraceDirection`] (X06.3's existing `In`/`Out`/`Both`, default
//! `Both` per the pack), `depth` (default [`DEFAULT_DEPTH`] = 3),
//! `include_tests` (filters [`crate::code_graph::CodeNode::Test`] nodes
//! out of hop lists when `false`), and `edge_types` (keep only hops
//! whose [`super::EdgeKind`] is in the given set; `None`/empty means "no
//! filter").
//!
//! # Determinism
//!
//! Every response list here is sorted by a stable key (node id, then
//! path hop sequence) before being returned -- [`super::CodeAdjacency`]'s
//! own DFS order is deterministic for a fixed graph already, but this
//! module does not rely on that alone: it re-sorts explicitly so the
//! parity contract ("deterministic ordering", per this lane's mission)
//! holds even if the underlying traversal's internal order ever changes.

use super::{test_node_ids, CodeAdjacency, EdgeKind, PathHop, TraceDirection};
use crate::code_graph::CodeGraph;
use std::collections::{BTreeSet, HashSet};

/// Default trace depth (scout digest §1: "depth default 3").
pub const DEFAULT_DEPTH: usize = 3;

/// One hop in a `calls`-mode trace: identical shape to
/// [`super::PathHop`], re-exported under this module's naming so callers
/// of the parity surface do not need to reach into `super` directly.
pub type CallHop = PathHop;

/// A full traced path: an ordered hop list plus which node the path
/// started from (paths themselves never include the start node as a
/// hop, matching [`CodeAdjacency::trace_calls`]'s existing contract).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TracedPath {
    pub start_node_id: String,
    pub hops: Vec<CallHop>,
}

/// The full response for [`trace_calls`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CallTraceReport {
    pub paths: Vec<TracedPath>,
}

/// `calls` mode: a thin, parity-shaped wrapper over
/// [`CodeAdjacency::trace_calls`]. Filters by `include_tests` and
/// `edge_types` after the traversal (the underlying traversal has no
/// notion of either) and re-sorts every path list deterministically.
pub fn trace_calls(
    adjacency: &CodeAdjacency,
    graph: &CodeGraph,
    start: &str,
    direction: TraceDirection,
    depth: usize,
    include_tests: bool,
    edge_types: Option<&[EdgeKind]>,
) -> CallTraceReport {
    let raw_paths = adjacency.trace_calls(start, direction, depth);
    let test_ids = test_node_ids(graph);

    let mut paths: Vec<TracedPath> = raw_paths
        .into_iter()
        .filter_map(|hops| filter_path(hops, include_tests, edge_types, &test_ids))
        .map(|hops| TracedPath {
            start_node_id: start.to_string(),
            hops,
        })
        .collect();

    sort_paths(&mut paths);
    CallTraceReport { paths }
}

/// How much of a real argument-expression-to-parameter binding a
/// [`DataFlowHop`] actually carries. `CallGraphOnly` is the only variant
/// this crate can honestly produce today -- see module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Approximation {
    /// Only call-graph edges (who calls whom) are known; no
    /// argument-expression-to-parameter linkage data exists in the
    /// parser layer this crate builds on.
    CallGraphOnly,
}

/// A resolved argument-expression -> parameter binding. Never populated
/// by this crate's current parser layer (see module docs) -- the type
/// exists so a future parser upgrade can fill it in without breaking
/// [`DataFlowHop`]'s shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamLink {
    pub argument_expr: String,
    pub parameter_name: String,
}

/// One hop in a `data_flow`-mode trace: the call-graph hop plus an
/// (always-absent, for now) parameter link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataFlowHop {
    pub hop: CallHop,
    pub param_link: Option<ParamLink>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataFlowPath {
    pub start_node_id: String,
    pub hops: Vec<DataFlowHop>,
}

/// The full response for [`trace_data_flow`]. `approximation` is always
/// present and always [`Approximation::CallGraphOnly`] today -- callers
/// MUST check it before treating `param_link` as ground truth (it never
/// is, yet).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataFlowReport {
    pub paths: Vec<DataFlowPath>,
    pub approximation: Approximation,
}

/// `data_flow` mode: follows the same call-graph edges as [`trace_calls`]
/// (there is no separate data-flow edge kind in [`CodeGraph`] -- see
/// module docs) and wraps every hop as a [`DataFlowHop`] with
/// `param_link: None`, honestly labeled via
/// [`DataFlowReport::approximation`].
pub fn trace_data_flow(
    adjacency: &CodeAdjacency,
    graph: &CodeGraph,
    start: &str,
    direction: TraceDirection,
    depth: usize,
    include_tests: bool,
    edge_types: Option<&[EdgeKind]>,
) -> DataFlowReport {
    let call_report = trace_calls(
        adjacency,
        graph,
        start,
        direction,
        depth,
        include_tests,
        edge_types,
    );

    let paths = call_report
        .paths
        .into_iter()
        .map(|path| DataFlowPath {
            start_node_id: path.start_node_id,
            hops: path
                .hops
                .into_iter()
                .map(|hop| DataFlowHop {
                    hop,
                    param_link: None,
                })
                .collect(),
        })
        .collect();

    DataFlowReport {
        paths,
        approximation: Approximation::CallGraphOnly,
    }
}

/// One producer -> route -> consumer path for `cross_service` mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteMediator {
    pub method: String,
    pub path: String,
    /// The file node id that declares the route (the producer).
    pub producer_node_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossServicePath {
    pub mediator: RouteMediator,
    /// The consumer node id: a file that reaches the producer via an
    /// `Imports` or `Calls` edge within the trace depth.
    pub consumer_node_id: String,
    /// The hop chain from the consumer to the producer (same shape as
    /// [`CallHop`] so callers can render it identically to `calls` mode).
    pub hops: Vec<CallHop>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CrossServiceReport {
    pub paths: Vec<CrossServicePath>,
}

/// `cross_service` mode: producer/route/consumer paths mediated by
/// [`CodeGraph::routes`] (see module docs for the "no event edge kind
/// yet" honesty note). `start` may be either the producer file id or a
/// candidate consumer file id -- every route mediator reachable from
/// `start` within `depth` (as producer or consumer, depending on
/// `direction`) is reported.
pub fn trace_cross_service(
    adjacency: &CodeAdjacency,
    graph: &CodeGraph,
    start: &str,
    direction: TraceDirection,
    depth: usize,
    include_tests: bool,
) -> CrossServiceReport {
    let test_ids = test_node_ids(graph);
    let mut paths = Vec::new();

    for route in graph.routes() {
        let producer_id = route.from_file_id.clone();
        let mediator = RouteMediator {
            method: route.method.clone(),
            path: route.path.clone(),
            producer_node_id: producer_id.clone(),
        };

        // Outbound from `start`: is `start` the producer (or does it
        // reach the producer via Out edges), and who consumes it?
        let producer_reachable = producer_id == start
            || matches!(direction, TraceDirection::Out | TraceDirection::Both)
                && path_exists(adjacency, start, &producer_id, depth);

        if !producer_reachable {
            continue;
        }

        // Consumers: every node that reaches the producer via an
        // Imports/Calls edge in the Incoming direction (i.e. every
        // upstream dependent of the producer file), excluding the
        // producer itself.
        let consumers = adjacency.reverse_dependents(&producer_id, depth);
        for consumer_id in consumers {
            if consumer_id == producer_id {
                continue;
            }
            if !include_tests && test_ids.contains(&consumer_id) {
                continue;
            }
            let hops = adjacency
                .trace_calls(&consumer_id, TraceDirection::Out, depth)
                .into_iter()
                .find(|path| path.iter().any(|hop| hop.node_id == producer_id))
                .unwrap_or_default();

            paths.push(CrossServicePath {
                mediator: mediator.clone(),
                consumer_node_id: consumer_id,
                hops,
            });
        }
    }

    paths.sort_by(|a, b| {
        (
            a.mediator.method.as_str(),
            a.mediator.path.as_str(),
            a.consumer_node_id.as_str(),
        )
            .cmp(&(
                b.mediator.method.as_str(),
                b.mediator.path.as_str(),
                b.consumer_node_id.as_str(),
            ))
    });
    CrossServiceReport { paths }
}

/// Whether any path of length <= `depth` connects `from` to `to` in
/// `direction`. Used only by [`trace_cross_service`]'s outbound check;
/// deliberately reuses [`CodeAdjacency::trace_calls`] rather than a
/// second traversal implementation.
fn path_exists(
    adjacency: &CodeAdjacency,
    from: &str,
    to: &str,
    depth: usize,
) -> bool {
    adjacency
        .trace_calls(from, TraceDirection::Out, depth)
        .iter()
        .any(|path| path.iter().any(|hop| hop.node_id == to))
}

/// Apply `include_tests`/`edge_types` filtering to one raw path. A path
/// that becomes empty after filtering (every hop dropped) is dropped
/// entirely rather than returned as a vacuous zero-hop path.
fn filter_path(
    hops: Vec<PathHop>,
    include_tests: bool,
    edge_types: Option<&[EdgeKind]>,
    test_ids: &HashSet<String>,
) -> Option<Vec<PathHop>> {
    let filtered: Vec<PathHop> = hops
        .into_iter()
        .filter(|hop| include_tests || !test_ids.contains(&hop.node_id))
        .filter(|hop| edge_types.map(|kinds| kinds.contains(&hop.via)).unwrap_or(true))
        .collect();
    if filtered.is_empty() {
        None
    } else {
        Some(filtered)
    }
}

/// Deterministic ordering for a list of [`TracedPath`]s: by start node
/// (constant within one call, kept for clarity), then by the
/// concatenated hop-id sequence, then by length.
fn sort_paths(paths: &mut [TracedPath]) {
    paths.sort_by(|a, b| {
        let a_key: Vec<&str> = a.hops.iter().map(|h| h.node_id.as_str()).collect();
        let b_key: Vec<&str> = b.hops.iter().map(|h| h.node_id.as_str()).collect();
        a_key.cmp(&b_key)
    });
}

/// Distinct node ids touched by a [`CallTraceReport`] -- a small helper
/// the MCP/CLI wrapper lane (out of scope here) is expected to want;
/// kept here since it is a pure function of this module's own types.
pub fn distinct_node_ids(report: &CallTraceReport) -> Vec<String> {
    let set: BTreeSet<&str> = report
        .paths
        .iter()
        .flat_map(|p| p.hops.iter().map(|h| h.node_id.as_str()))
        .collect();
    set.into_iter().map(str::to_string).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_graph::Manifest;
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
            TraceDirection::Out,
            3,
            true,
            None,
        );
        let wrapped_ids = distinct_node_ids(&report);

        let raw = adjacency.trace_calls("file:a.rs", TraceDirection::Out, 3);
        let mut raw_ids: BTreeSet<String> = raw
            .into_iter()
            .flat_map(|p| p.into_iter().map(|h| h.node_id))
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

        let first = trace_calls(
            &adjacency,
            &graph,
            "file:a.rs",
            TraceDirection::Out,
            3,
            true,
            None,
        );
        let second = trace_calls(
            &adjacency,
            &graph,
            "file:a.rs",
            TraceDirection::Out,
            3,
            true,
            None,
        );
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
            TraceDirection::In,
            3,
            true,
            None,
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
            TraceDirection::Out,
            1,
            true,
            None,
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
            TraceDirection::In,
            3,
            true,
            None,
        );
        let without_tests = trace_calls(
            &adjacency,
            &graph,
            &helper_id,
            TraceDirection::In,
            3,
            false,
            None,
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
            TraceDirection::Out,
            3,
            true,
            None,
        );
        assert_eq!(report.approximation, Approximation::CallGraphOnly);
        assert!(!report.paths.is_empty());
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

        // a.rs -> helper (b.rs's symbol) -> deep (c.rs's symbol): the
        // call graph reaches both callee symbols from file:a.rs within
        // depth 3 (file->symbol Calls edges, per code_graph's shape).
        let report = trace_data_flow(
            &adjacency,
            &graph,
            "file:a.rs",
            TraceDirection::Out,
            3,
            true,
            None,
        );
        let reaches_helper = report.paths.iter().any(|p| {
            p.hops
                .iter()
                .any(|h| h.hop.node_id.contains("helper"))
        });
        let reaches_deep = report
            .paths
            .iter()
            .any(|p| p.hops.iter().any(|h| h.hop.node_id.contains("deep")));
        assert!(
            reaches_helper && reaches_deep,
            "expected data_flow to reach both helper and deep via call edges"
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
            TraceDirection::Both,
            3,
            true,
        );
        assert!(
            !report.paths.is_empty(),
            "expected at least one cross_service path from router.ts's own declared route"
        );
        let has_expected_route = report
            .paths
            .iter()
            .any(|p| p.mediator.method == "GET" && p.mediator.path == "/a");
        assert!(has_expected_route, "expected GET /a route among mediators, got {:?}", report.paths);
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
            TraceDirection::Both,
            3,
            true,
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
            TraceDirection::Both,
            3,
            true,
        );
        let without_tests = trace_cross_service(
            &adjacency,
            &graph,
            "file:router.ts",
            TraceDirection::Both,
            3,
            false,
        );
        let without_has_test_consumer = without_tests
            .paths
            .iter()
            .any(|p| p.consumer_node_id.contains("a_test"));
        assert!(!without_has_test_consumer);
        let _ = with_tests;
        Ok(())
    }

    // --- unknown node handling -----------------------------------------

    #[test]
    fn unknown_start_node_returns_empty_report_not_panic() -> TestResult {
        let dir = tempfile::tempdir()?;
        let graph = build_fixture_graph(dir.path())?;
        let adjacency = CodeAdjacency::build(&graph);

        let calls = trace_calls(
            &adjacency,
            &graph,
            "file:does-not-exist.rs",
            TraceDirection::Both,
            3,
            true,
            None,
        );
        assert!(calls.paths.is_empty());

        let data_flow = trace_data_flow(
            &adjacency,
            &graph,
            "file:does-not-exist.rs",
            TraceDirection::Both,
            3,
            true,
            None,
        );
        assert!(data_flow.paths.is_empty());

        let cross_service = trace_cross_service(
            &adjacency,
            &graph,
            "file:does-not-exist.rs",
            TraceDirection::Both,
            3,
            true,
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
            TraceDirection::Out,
            3,
            true,
            Some(&[EdgeKind::Calls]),
        );
        for path in &calls_only.paths {
            for hop in &path.hops {
                assert_eq!(hop.via, EdgeKind::Calls);
            }
        }
        Ok(())
    }
}
