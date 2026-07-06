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
//! # Parity risk labels (`risk_labels`)
//!
//! Baseline-source-verified correction (orchestrator, post-scout-digest
//! extraction of the actual C source): the baseline's `trace_path` --
//! not `detect_changes` -- is where its ONLY risk concept lives, gated
//! behind a `risk_labels: bool` request param (default `false`).  When
//! set, the baseline labels every hop by pure BFS hop-distance from the
//! traced root: hop 1 = `CRITICAL`, hop 2 = `HIGH`, hop 3 = `MEDIUM`,
//! the root itself or hop >= 4 = `LOW` (its `cbm_hop_to_risk`,
//! uppercase strings). [`trace_calls`] (and, by wrapping it,
//! [`trace_data_flow`]) reproduce exactly this when their own
//! `risk_labels` argument is `true`, via [`hop_to_risk_label`] and
//! [`TracedPath::risk_labels`] -- a parallel array to `hops`, index-for-
//! index, `None` when `risk_labels=false` was requested (never a
//! default-empty `Vec` masquerading as "no labels asked for"). This is
//! PARITY ONLY: it does not replace or feed into this crate's own,
//! richer [`crate::impact::RiskFactors`]-based classification, which
//! has no baseline counterpart at all (the baseline's `detect_changes`
//! carries zero risk fields) and remains a documented enforcer
//! extension, never conflated with the parity hop labels here.
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
/// `risk_labels` is `Some(_)` (one entry per `hops` entry, same index)
/// only when the caller passed `risk_labels: true`; `None` otherwise --
/// see the module docs' "Parity risk labels" section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TracedPath {
    pub start_node_id: String,
    pub hops: Vec<CallHop>,
    pub risk_labels: Option<Vec<RiskLabel>>,
}

/// The baseline's `cbm_hop_to_risk` labels, reproduced verbatim
/// (uppercase strings on the wire via [`RiskLabel::as_str`]) -- see the
/// module docs' "Parity risk labels" section. Named distinctly from
/// [`crate::impact::RiskLevel`] (three-tier, PascalCase, used for this
/// crate's own richer classification) so the two are never confused at
/// the type level, matching the orchestrator's "expose both, never
/// overwrite" directive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLabel {
    Critical,
    High,
    Medium,
    Low,
}

impl RiskLabel {
    pub fn as_str(&self) -> &'static str {
        match self {
            RiskLabel::Critical => "CRITICAL",
            RiskLabel::High => "HIGH",
            RiskLabel::Medium => "MEDIUM",
            RiskLabel::Low => "LOW",
        }
    }
}

/// The baseline's `cbm_hop_to_risk`: pure BFS hop-distance from the
/// traced root. `hop_number` is 1-indexed (the first hop away from
/// root is hop 1, matching a [`TracedPath`]'s `hops[0]`); the root node
/// itself is never passed here (paths never include it as a hop), so
/// "root" in the baseline's `root/4+ = LOW` is naturally covered by the
/// `_ => Low` arm at hop_number >= 4 -- there is no hop_number=0 case
/// to special-case separately.
pub fn hop_to_risk_label(hop_number: usize) -> RiskLabel {
    match hop_number {
        1 => RiskLabel::Critical,
        2 => RiskLabel::High,
        3 => RiskLabel::Medium,
        _ => RiskLabel::Low,
    }
}

/// The full response for [`trace_calls`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CallTraceReport {
    pub paths: Vec<TracedPath>,
}

/// Every `calls`-mode request param besides the `(adjacency, graph,
/// start)` subject triple, bundled so [`trace_calls`] stays under
/// clippy's default too-many-arguments threshold without an `#[allow]`
/// (same posture as this crate's `DfsPathState`/`RelatedWalkState`
/// bundling in [`super`]). `Default` gives every field its
/// parity-documented default (`direction: Both`, `depth:
/// DEFAULT_DEPTH`, `include_tests: true`, `edge_types: None`,
/// `risk_labels: false`) so a caller that only needs a couple of
/// non-default fields can use struct-update syntax.
#[derive(Debug, Clone)]
pub struct TraceCallsParams<'a> {
    pub direction: TraceDirection,
    pub depth: usize,
    pub include_tests: bool,
    pub edge_types: Option<&'a [EdgeKind]>,
    pub risk_labels: bool,
}

impl Default for TraceCallsParams<'_> {
    fn default() -> Self {
        Self {
            direction: TraceDirection::Both,
            depth: DEFAULT_DEPTH,
            include_tests: true,
            edge_types: None,
            risk_labels: false,
        }
    }
}

/// `calls` mode: a thin, parity-shaped wrapper over
/// [`CodeAdjacency::trace_calls`]. Filters by `params.include_tests`
/// and `params.edge_types` after the traversal (the underlying
/// traversal has no notion of either), re-sorts every path list
/// deterministically, and -- when `params.risk_labels` is `true` --
/// attaches the baseline's hop-distance risk label to every hop (see
/// module docs).
pub fn trace_calls(
    adjacency: &CodeAdjacency,
    graph: &CodeGraph,
    start: &str,
    params: &TraceCallsParams<'_>,
) -> CallTraceReport {
    let raw_paths = adjacency.trace_calls(start, params.direction, params.depth);
    let test_ids = test_node_ids(graph);

    let mut paths: Vec<TracedPath> = raw_paths
        .into_iter()
        .filter_map(|hops| filter_path(hops, params.include_tests, params.edge_types, &test_ids))
        .map(|hops| {
            let labels = params
                .risk_labels
                .then(|| (1..=hops.len()).map(hop_to_risk_label).collect::<Vec<_>>());
            TracedPath {
                start_node_id: start.to_string(),
                hops,
                risk_labels: labels,
            }
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
/// [`DataFlowReport::approximation`]. Takes the same [`TraceCallsParams`]
/// bundle as [`trace_calls`] (including `risk_labels`, which is dropped
/// here -- [`DataFlowHop`] has no risk-label field, since the baseline's
/// risk-labels concept is specific to its `calls`-shaped BFS response,
/// not `data_flow`'s; a caller that wants risk labels for a data_flow
/// walk should call [`trace_calls`] directly with the same params).
pub fn trace_data_flow(
    adjacency: &CodeAdjacency,
    graph: &CodeGraph,
    start: &str,
    params: &TraceCallsParams<'_>,
) -> DataFlowReport {
    let call_report = trace_calls(adjacency, graph, start, params);

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

/// Every [`trace_cross_service`] request param besides the `(adjacency,
/// graph, start)` subject triple, bundled so the function stays under
/// clippy's default too-many-arguments threshold without an `#[allow]`
/// (same posture as [`TraceCallsParams`]/this crate's `DfsPathState`/
/// `RelatedWalkState` bundling in [`super`] -- this crate runs clippy
/// with zero `#[allow(clippy::…)]`, per the workpack gate).
#[derive(Debug, Clone, Copy)]
pub struct TraceCrossServiceParams {
    pub direction: TraceDirection,
    pub depth: usize,
    pub include_tests: bool,
}

/// `cross_service` mode: producer/route/consumer paths mediated by
/// [`CodeGraph::routes`] (see module docs for the "no event edge kind
/// yet" honesty note). `start` may be either the producer file id or a
/// candidate consumer file id -- every route mediator reachable from
/// `start` within `params.depth` (as producer or consumer, depending on
/// `params.direction`) is reported.
pub fn trace_cross_service(
    adjacency: &CodeAdjacency,
    graph: &CodeGraph,
    start: &str,
    params: TraceCrossServiceParams,
) -> CrossServiceReport {
    let TraceCrossServiceParams {
        direction,
        depth,
        include_tests,
    } = params;
    let test_ids = test_node_ids(graph);
    let mut paths = Vec::new();

    for route in graph.routes() {
        let producer_id = route.from_file_id.clone();
        let mediator = RouteMediator {
            method: route.method.clone(),
            path: route.path.clone(),
            producer_node_id: producer_id.clone(),
        };

        // Consumers: every node that reaches the producer via an
        // Imports/Calls edge in the Incoming direction (i.e. every
        // upstream dependent of the producer file), excluding the
        // producer itself. Computed once per route regardless of
        // `direction` -- `direction` only gates *which* relationship
        // `start` must have to this route before it is reported (see
        // below), not how consumers themselves are discovered.
        let consumers = adjacency.reverse_dependents(&producer_id, depth);

        // `start`'s relationship to this route depends on `direction`,
        // mirroring `trace_calls`'s own Out/In/Both contract:
        // - Out: `start` is the producer, or reaches the producer via
        //   its own outbound (Calls/Imports) edges -- "starting from
        //   `start`, what do I produce/call downstream".
        // - In: `start` is the producer, or `start` is itself one of
        //   the producer's consumers -- "starting from `start`, what do
        //   I consume upstream".
        // - Both: either of the above.
        let reaches_as_out = matches!(direction, TraceDirection::Out | TraceDirection::Both)
            && path_exists(adjacency, start, &producer_id, depth);
        let reaches_as_in = matches!(direction, TraceDirection::In | TraceDirection::Both)
            && consumers.iter().any(|id| id == start);
        let producer_reachable = producer_id == start || reaches_as_out || reaches_as_in;

        if !producer_reachable {
            continue;
        }

        let mut emitted_for_route = false;
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
            emitted_for_route = true;
        }

        // A declared route is itself a real fact about the producer file,
        // independent of whether any other file consumes it yet (e.g. a
        // freshly declared route with no callers indexed so far). When
        // `start` reaches the route (as producer or consumer above) but no
        // external consumer path was found, still surface the mediator so
        // callers can discover the route exists -- self-referential
        // (producer as its own "consumer", zero hops) rather than silently
        // dropping the route from the report.
        if !emitted_for_route {
            paths.push(CrossServicePath {
                mediator: mediator.clone(),
                consumer_node_id: producer_id.clone(),
                hops: Vec::new(),
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
fn path_exists(adjacency: &CodeAdjacency, from: &str, to: &str, depth: usize) -> bool {
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
        .filter(|hop| {
            edge_types
                .map(|kinds| kinds.contains(&hop.via))
                .unwrap_or(true)
        })
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
