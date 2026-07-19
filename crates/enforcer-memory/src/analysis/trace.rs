//! X06.P2: parity `trace_path` -- the three trace modes the baseline's
//! `trace_path` tool exposes (scout digest section 1, row 4: "modes
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
//!   `calls` mode (now including [`MemoryEdgeKind::DataFlows`], the
//!   symbol-scoped edge [`super::CodeAdjacency::build`] adds alongside
//!   [`MemoryEdgeKind::Calls`] for every resolved call that has
//!   captured argument expressions -- see [`crate::data_flow`]'s module
//!   doc for the baseline C source this mirrors and exactly why it stops
//!   short of the baseline's route-mediated closure), and additionally
//!   populates each [`DataFlowHop`] with a [`ParamLink`] when
//!   [`crate::data_flow::materialize`] found an argument expression for
//!   the hop's target. This crate's parser layer still records no
//!   callee parameter *names* ([`crate::code_graph::SymbolNode`] has no
//!   parameter list -- see [`crate::data_flow`]'s docs for exactly why),
//!   so [`ParamLink::parameter_name`] stays `None`: only
//!   [`ParamLink::argument_expr`] is ever populated, and only for a hop
//!   whose target has a matching [`crate::data_flow::DataFlowEdge`] (a
//!   hop with no captured arguments, or whose call was
//!   [`resolution::ResolutionConfidence::Unresolved`], keeps
//!   `param_link: None` -- never a fabricated linkage).
//!   [`DataFlowReport::approximation`] is fixed at
//!   [`Approximation::CallGraphOnly`] regardless, since even a populated
//!   `argument_expr` is call-graph-plus-raw-argument-text, not a real
//!   argument-to-parameter *binding* -- a caller must still not mistake
//!   this for type-checked dataflow.
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
//! whose [`MemoryEdgeKind`] is in the given set; `None`/empty means "no
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

use super::{test_node_ids, CodeAdjacency, PathHop};
use crate::code_graph::CodeGraph;
use crate::owned_boundary::{Retained, RetainedDisplay};
use enforcer_domain::memory_types::{
    Approximation, MemoryAnalysisNodeId, MemoryEdgeKind, MemoryResolutionSymbolId, RiskLabel,
    TraceArgumentExpression, TraceDepth, TraceDirection, TraceIncludeTests, TraceNodeId,
    TraceParameterName, TracePathExists, TraceRiskLabels, TraceRouteMethod, TraceRoutePath,
};
use std::collections::{BTreeSet, HashSet};

/// Default trace depth (scout digest section 1: "depth default 3").
pub const DEFAULT_DEPTH: usize = 3;

/// A full traced path: an ordered hop list plus which node the path
/// started from (paths themselves never include the start node as a
/// hop, matching [`CodeAdjacency::trace_calls`]'s existing contract).
/// `risk_labels` is `Some(_)` (one entry per `hops` entry, same index)
/// only when the caller passed `risk_labels: true`; `None` otherwise --
/// see the module docs' "Parity risk labels" section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TracedPath {
    pub start_node_id: TraceNodeId,
    pub hops: Vec<PathHop>,
    pub risk_labels: Option<Vec<RiskLabel>>,
}

/// The baseline's `cbm_hop_to_risk` labels, reproduced verbatim
/// (uppercase strings on the wire via [`RiskLabel::as_str`]) -- see the
/// module docs' "Parity risk labels" section. Named distinctly from
/// [`enforcer_domain::memory_types::RiskLevel`] (three-tier, PascalCase, used for this
/// crate's own richer classification) so the two are never confused at
/// the type level, matching the orchestrator's "expose both, never
/// overwrite" directive.
/// The baseline's `cbm_hop_to_risk`: pure BFS hop-distance from the
/// traced root. `hop_number` is 1-indexed (the first hop away from
/// root is hop 1, matching a [`TracedPath`]'s `hops[0]`); the root node
/// itself is never passed here (paths never include it as a hop), so
/// "root" in the baseline's `root/4+ = LOW` is naturally covered by the
/// `_ => Low` arm at hop_number >= 4 -- there is no hop_number=0 case
/// to special-case separately.
pub fn hop_to_risk_label(hop_number: impl Into<TraceDepth>) -> RiskLabel {
    match hop_number.into().get() {
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
    pub depth: TraceDepth,
    pub include_tests: TraceIncludeTests,
    pub edge_types: Option<&'a [MemoryEdgeKind]>,
    pub risk_labels: TraceRiskLabels,
}

impl Default for TraceCallsParams<'_> {
    fn default() -> Self {
        Self {
            direction: TraceDirection::Both,
            depth: DEFAULT_DEPTH.into(),
            include_tests: true.into(),
            edge_types: None,
            risk_labels: false.into(),
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
    start: impl Into<TraceNodeId>,
    params: &TraceCallsParams<'_>,
) -> CallTraceReport {
    let start = start.into();
    let start = start.as_str();
    let raw_paths = adjacency.trace_calls(start, params.direction, params.depth.get());
    let test_ids = test_node_ids(graph);

    let mut paths: Vec<TracedPath> = raw_paths
        .into_iter()
        .filter_map(|hops| filter_path(hops, params.include_tests, params.edge_types, &test_ids))
        .map(|hops| {
            let labels = params
                .risk_labels
                .includes_risk_labels()
                .then(|| (1..=hops.len()).map(hop_to_risk_label).collect::<Vec<_>>());
            TracedPath {
                start_node_id: start.retained_display().into(),
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
/// A call site's captured argument expression, optionally paired with
/// the parameter name it binds to. `parameter_name` is never populated
/// by this crate's current parser layer (see module docs and
/// [`crate::data_flow`]'s docs for exactly why) -- the field exists so a
/// future parser upgrade that captures callee parameter names can fill
/// it in without another [`DataFlowHop`] shape break.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamLink {
    pub argument_expr: TraceArgumentExpression,
    pub parameter_name: Option<TraceParameterName>,
}

/// One hop in a `data_flow`-mode trace: the call-graph hop plus a
/// [`ParamLink`] when [`crate::data_flow::materialize`] found a captured
/// argument expression for a call resolving to this hop's node (`None`
/// when it did not -- e.g. the hop's target has no argument data, or
/// this hop is an `Imports`-kind hop rather than a `Calls`-kind one; see
/// [`trace_data_flow`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataFlowHop {
    pub hop: PathHop,
    pub param_link: Option<ParamLink>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataFlowPath {
    pub start_node_id: TraceNodeId,
    pub hops: Vec<DataFlowHop>,
}

/// The full response for [`trace_data_flow`]. `approximation` is always
/// present and always [`Approximation::CallGraphOnly`] today -- callers
/// MUST check it before treating a hop's [`ParamLink::argument_expr`] as
/// a real argument-to-parameter *binding* (it never is: it is call-graph
/// resolution plus raw captured argument text, matching the baseline's
/// own `caller_args` granularity -- see [`crate::data_flow`]'s module
/// docs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataFlowReport {
    pub paths: Vec<DataFlowPath>,
    pub approximation: Approximation,
}

/// `data_flow` mode: follows the same call-graph edges as [`trace_calls`]
/// (there is no separate data-flow edge kind in [`CodeGraph`] -- see
/// module docs), then attaches a [`ParamLink`] to every hop whose node id
/// [`crate::data_flow::materialize`] recorded at least one argument
/// expression flowing into it (via
/// [`crate::data_flow::argument_exprs_by_target`], keyed by resolved
/// target symbol id -- the same id space [`PathHop::node_id`] uses). A
/// hop with no matching [`crate::data_flow::DataFlowEdge`] (no captured
/// arguments for any call resolving to it, or an `Imports`-kind hop that
/// was never a call at all) keeps `param_link: None`, never a fabricated
/// link. Takes the same [`TraceCallsParams`] bundle as [`trace_calls`]
/// (including `risk_labels`, which is dropped here -- [`DataFlowHop`] has
/// no risk-label field, since the baseline's risk-labels concept is
/// specific to its `calls`-shaped BFS response, not `data_flow`'s; a
/// caller that wants risk labels for a data_flow walk should call
/// [`trace_calls`] directly with the same params).
pub fn trace_data_flow(
    adjacency: &CodeAdjacency,
    graph: &CodeGraph,
    start: impl Into<TraceNodeId>,
    params: &TraceCallsParams<'_>,
) -> DataFlowReport {
    let start = start.into();
    let start = start.as_str();
    let call_report = trace_calls(adjacency, graph, start, params);
    let data_flow_graph = crate::data_flow::materialize(graph);
    let args_by_target = crate::data_flow::argument_exprs_by_target(&data_flow_graph);

    let paths = call_report
        .paths
        .into_iter()
        .map(|path| DataFlowPath {
            start_node_id: path.start_node_id,
            hops: path
                .hops
                .into_iter()
                .map(|hop| {
                    let param_link = args_by_target.get(hop.node_id.as_str()).and_then(|exprs| {
                        exprs.first().map(|expr| ParamLink {
                            argument_expr: expr.retained_display().into(),
                            parameter_name: None,
                        })
                    });
                    DataFlowHop { hop, param_link }
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
    pub method: TraceRouteMethod,
    pub path: TraceRoutePath,
    /// The file node id that declares the route (the producer).
    pub producer_node_id: TraceNodeId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossServicePath {
    pub mediator: RouteMediator,
    /// The consumer node id: a file that reaches the producer via an
    /// `Imports` or `Calls` edge within the trace depth.
    pub consumer_node_id: TraceNodeId,
    /// The hop chain from the consumer to the producer (same shape as
    /// [`PathHop`] so callers can render it identically to `calls` mode).
    pub hops: Vec<PathHop>,
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
/// with zero `#[allow(clippy::...)]`, per the workpack gate).
#[derive(Debug, Clone, Copy)]
pub struct TraceCrossServiceParams {
    pub direction: TraceDirection,
    pub depth: TraceDepth,
    pub include_tests: TraceIncludeTests,
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
    start: impl Into<TraceNodeId>,
    params: TraceCrossServiceParams,
) -> CrossServiceReport {
    let start = start.into();
    let TraceCrossServiceParams {
        direction,
        depth,
        include_tests,
    } = params;
    let test_ids = test_node_ids(graph);
    let mut paths = Vec::new();

    for route in graph.routes() {
        let producer_id: MemoryAnalysisNodeId = route.from_file_id.as_str().into();
        let start_analysis: MemoryAnalysisNodeId = start.as_str().into();
        let mediator = RouteMediator {
            method: route.method.as_str().retained().into(),
            path: route.path.as_str().retained().into(),
            producer_node_id: producer_id.as_str().retained().into(),
        };

        // Consumers: every node that reaches the producer via an
        // Imports/Calls edge in the Incoming direction (i.e. every
        // upstream dependent of the producer file), excluding the
        // producer itself. Computed once per route regardless of
        // `direction` -- `direction` only gates *which* relationship
        // `start` must have to this route before it is reported (see
        // below), not how consumers themselves are discovered.
        let consumers = adjacency.reverse_dependents(producer_id.as_str(), depth.get());

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
            && path_exists(adjacency, &start_analysis, &producer_id, depth).exists();
        let reaches_as_in = matches!(direction, TraceDirection::In | TraceDirection::Both)
            && consumers.iter().any(|id| id == start.as_str());
        let producer_reachable =
            producer_id.as_str() == start.as_str() || reaches_as_out || reaches_as_in;

        if !producer_reachable {
            continue;
        }

        let mut emitted_for_route = false;
        for consumer_id in consumers {
            if consumer_id == producer_id {
                continue;
            }
            if !include_tests.includes_tests()
                && test_ids.contains(&MemoryResolutionSymbolId::from(consumer_id.as_str()))
            {
                continue;
            }
            let hops = adjacency
                .trace_calls(consumer_id.retained(), TraceDirection::Out, depth.get())
                .into_iter()
                .find(|path| path.iter().any(|hop| hop.node_id == producer_id))
                .unwrap_or_else(Vec::new);

            paths.push(CrossServicePath {
                mediator: mediator.retained(),
                consumer_node_id: consumer_id.as_str().into(),
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
                mediator: mediator.retained(),
                consumer_node_id: producer_id.as_str().retained().into(),
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
fn path_exists(
    adjacency: &CodeAdjacency,
    from: &MemoryAnalysisNodeId,
    to: &MemoryAnalysisNodeId,
    depth: TraceDepth,
) -> TracePathExists {
    adjacency
        .trace_calls(from.as_str(), TraceDirection::Out, depth.get())
        .iter()
        .any(|path| path.iter().any(|hop| hop.node_id == to.as_str()))
        .into()
}

/// Apply `include_tests`/`edge_types` filtering to one raw path. A path
/// that becomes empty after filtering (every hop dropped) is dropped
/// entirely rather than returned as a vacuous zero-hop path.
fn filter_path(
    hops: Vec<PathHop>,
    include_tests: TraceIncludeTests,
    edge_types: Option<&[MemoryEdgeKind]>,
    test_ids: &HashSet<MemoryResolutionSymbolId>,
) -> Option<Vec<PathHop>> {
    let filtered: Vec<PathHop> = hops
        .into_iter()
        .filter(|hop| {
            include_tests.includes_tests()
                || !test_ids.contains(&MemoryResolutionSymbolId::from(hop.node_id.as_str()))
        })
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
        a.hops
            .iter()
            .map(|h| h.node_id.as_str())
            .cmp(b.hops.iter().map(|h| h.node_id.as_str()))
    });
}

/// Distinct node ids touched by a [`CallTraceReport`] -- a small helper
/// the MCP/CLI wrapper lane (out of scope here) is expected to want;
/// kept here since it is a pure function of this module's own types.
pub fn distinct_node_ids(report: &CallTraceReport) -> Vec<TraceNodeId> {
    let set: BTreeSet<&str> = report
        .paths
        .iter()
        .flat_map(|p| p.hops.iter().map(|h| h.node_id.as_str()))
        .collect();
    set.into_iter().map(Into::into).collect()
}
