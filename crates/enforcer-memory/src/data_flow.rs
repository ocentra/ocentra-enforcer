//! X06 core parity: `DATA_FLOWS` edge materialization -- a post-index
//! pass that closes the gap [`crate::analysis::trace`] has documented
//! since it first shipped `trace_path`'s `data_flow` mode: "[`ParamLink`]
//! exists so a future parser upgrade that captures call-argument/
//! parameter lists can populate it... [but] there is no
//! argument-expression-to-parameter data in this crate to link" (see
//! `analysis/trace.rs` module docs). [`crate::resolution`] and
//! [`crate::code_graph::CallEdge::arg_texts`] have since landed exactly
//! that data (X06 type-aware resolution + cross-repo-intelligence); this
//! module is the missing wire between them.
//!
//! # Baseline semantics (`codebase-memory-mcp`)
//!
//! The baseline's `DATA_FLOWS` edge (`src/pipeline/pass_route_nodes.c`,
//! `create_data_flows`/`try_create_data_flow`, ~L590-L800) is a
//! *route-mediated call-graph closure*: for every `Route` node, every
//! caller reaching it via an `HTTP_CALLS`/`ASYNC_CALLS` edge is paired
//! with every handler reaching it via a `HANDLES` edge, and a new
//! `caller -> DATA_FLOWS -> handler` edge is inserted (skipped when
//! `caller == handler`, or when a direct `CALLS` edge between the two
//! already exists -- `has_direct_call`, L594-L604). The inserted edge's
//! properties (`finish_data_flow_props`, L647-L670) carry `via`
//! (route name), `route` (route qualified name), `edge_type` (the
//! original `HTTP_CALLS`/`ASYNC_CALLS`), `via_infra` (bool, only when the
//! handler was reached through an `INFRA_MAPS` hop rather than a direct
//! `HANDLES`), `handler_params` (the handler function node's
//! `param_names` property array) and `caller_args` (the raw `args` JSON
//! fragment already recorded on the caller's `HTTP_CALLS`/`ASYNC_CALLS`
//! edge). `trace_path mode=data_flow` (`src/mcp/mcp.c` L2659,
//! `mode_data_flow = {"CALLS", "DATA_FLOWS"}`) then walks `CALLS` and
//! `DATA_FLOWS` edges together so a trace can hop across an HTTP/async
//! boundary exactly like a normal call.
//!
//! # Why this crate cannot reproduce that mechanism verbatim
//!
//! `Route`/`HANDLES`/`HTTP_CALLS`/`ASYNC_CALLS`/`INFRA_MAPS` are not
//! concepts [`crate::code_graph::CodeGraph`] has: its only route concept
//! is [`crate::code_graph::RouteEdge`] (`from_file_id` declares a route,
//! consumed by [`crate::analysis::trace::trace_cross_service`] already),
//! with no handler-linking edge and no HTTP/async-call classification on
//! [`crate::code_graph::CallEdge`]. Inventing that whole edge-kind
//! vocabulary to chase the baseline's exact mechanism would be a much
//! larger, differently-scoped slice than "materialize DATA_FLOW edges"
//! -- and would fabricate route/handler linkage this crate's extractors
//! do not actually detect, which is exactly the kind of silent, false
//! precision this crate's documented posture (`resolution.rs`,
//! `analysis/trace.rs`) refuses to produce elsewhere.
//!
//! What this module *does* reproduce, honestly, is the baseline's other
//! half: linking a call site's captured argument expressions
//! ([`crate::code_graph::CallEdge::arg_texts`], the direct analog of the
//! baseline's `caller_args`) to the call's resolved callee
//! ([`crate::resolution::ResolvedCall`], this crate's analog of "the
//! target this DATA_FLOWS edge points at"). This is the caller-argument
//! side of dataflow the baseline's own `try_create_data_flow` also just
//! copies as raw JSON text (`args_json`, `find_args_in_props`, L636-L645)
//! rather than binding to individual parameters by name -- so a
//! [`DataFlowEdge`] here is not a lesser version of the baseline's
//! linkage, it is the same granularity, minus the baseline's separate
//! route-closure step this crate's graph model cannot yet represent.
//! [`ParamLink::parameter_name`] stays `None` (never fabricated) because
//! no extractor in [`crate::languages`] records a callee's parameter
//! *names* anywhere on [`crate::code_graph::SymbolNode`] -- the same
//! honest gap [`crate::analysis::trace`] already documented; this module
//! narrows that gap (`argument_expr` is now real) without pretending to
//! close the part of it that still has no source data behind it.
//!
//! # What this module adds
//!
//! [`materialize`] runs [`crate::resolution::resolve`] (or reuses an
//! already-computed [`crate::code_graph::CodeGraph::resolved_calls`]
//! slice, index-aligned with [`crate::code_graph::CodeGraph::calls`] per
//! that field's own doc comment) and pairs each resolved call with its
//! [`crate::code_graph::CallEdge::arg_texts`], producing one
//! [`DataFlowEdge`] per call that has both a resolved target and at
//! least one captured argument expression -- a call with zero arguments
//! or with [`enforcer_domain::memory_types::ResolutionConfidence::Unresolved`]
//! produces no edge (never a fabricated empty/guessed one).
//! [`edges_from_symbol`] and [`edges_to_symbol`] index the result by
//! caller/callee for [`crate::analysis::trace`] (or any other caller) to
//! look up without re-scanning the whole edge list per query.
//!
//! [`crate::analysis::CodeAdjacency::build`] also calls [`materialize`]
//! directly and inserts one [`enforcer_domain::memory_types::MemoryEdgeKind::DataFlows`]
//! edge per [`DataFlowEdge`] that has a known `from_symbol_id`
//! (alongside, never instead of, the `Calls` edge it already adds for
//! the same resolved call) -- so a `trace_path` caller that requests
//! `edge_types: [DataFlows]` (or the unfiltered default) sees this
//! module's edges as first-class graph edges, the same way the baseline
//! exposes `DATA_FLOWS` as a real edge type rather than response-only
//! metadata.

use crate::code_graph::{CallEdge, CodeGraph};
use crate::owned_boundary::Retained;
use crate::resolution::{self, ResolvedCall};
use enforcer_domain::memory_types::{
    GraphSourceLine, MemoryDataFlowArgumentExpression, MemoryDataFlowSourceSymbolId,
    MemoryDataFlowTargetSymbolId, ResolutionConfidence,
};
use std::collections::HashMap;

/// One materialized data-flow edge: a call site's captured argument
/// expressions, linked to the specific resolved callee symbol they were
/// passed to. Mirrors the baseline's `DATA_FLOWS` edge properties
/// (`via`/`caller_args` in spirit -- see module docs) at the granularity
/// this crate's parser layer actually supports: call-graph resolution
/// plus raw argument text, never a fabricated parameter binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataFlowEdge {
    /// The enclosing symbol id the call site is lexically inside of
    /// ([`CallEdge::from_symbol`]-derived), if known. `None` for a
    /// module-scope call site -- kept rather than dropping the edge,
    /// since the argument data is still real even without a caller
    /// symbol id.
    pub from_symbol_id: Option<MemoryDataFlowSourceSymbolId>,
    /// The resolved target symbol id this call's arguments flow into.
    /// Never one of [`ResolvedCall::candidates`] picked arbitrarily when
    /// there is more than one -- an [`ResolutionConfidence::Ambiguous`]
    /// call produces one [`DataFlowEdge`] per candidate (see
    /// [`materialize`]), each carrying the same `argument_exprs`, so a
    /// consumer sees every real possibility rather than a silently
    /// narrowed guess.
    pub to_symbol_id: MemoryDataFlowTargetSymbolId,
    /// How confident [`resolution::resolve`] is in `to_symbol_id`,
    /// carried straight through so a consumer can filter on it exactly
    /// like [`ResolvedCall::confidence`] itself.
    pub confidence: ResolutionConfidence,
    /// The call site's argument expressions, in written order, straight
    /// from [`CallEdge::arg_texts`] -- the baseline's `caller_args`
    /// analog (see module docs).
    pub argument_exprs: Vec<MemoryDataFlowArgumentExpression>,
    /// The call site's source line ([`CallEdge::line`]), so a consumer
    /// can locate the edge back to its originating call without
    /// re-zipping against [`CodeGraph::calls`].
    pub line: GraphSourceLine,
}

/// The full result of one [`materialize`] pass: every [`DataFlowEdge`]
/// plus lookup indices so [`crate::analysis::trace`] (or any caller) can
/// find a symbol's outbound/inbound data-flow edges without re-scanning
/// [`Self::edges`] per query.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DataFlowGraph {
    edges: Vec<DataFlowEdge>,
}

impl DataFlowGraph {
    pub fn edges(&self) -> &[DataFlowEdge] {
        &self.edges
    }

    /// Every edge whose [`DataFlowEdge::from_symbol_id`] equals
    /// `symbol_id`, in [`Self::edges`] order.
    pub fn edges_from_symbol<'a>(
        &'a self,
        symbol_id: &'a MemoryDataFlowSourceSymbolId,
    ) -> impl Iterator<Item = &'a DataFlowEdge> {
        self.edges
            .iter()
            .filter(move |edge| edge.from_symbol_id.as_ref() == Some(symbol_id))
    }

    /// Every edge whose [`DataFlowEdge::to_symbol_id`] equals
    /// `symbol_id`, in [`Self::edges`] order.
    pub fn edges_to_symbol<'a>(
        &'a self,
        symbol_id: &'a MemoryDataFlowTargetSymbolId,
    ) -> impl Iterator<Item = &'a DataFlowEdge> {
        self.edges
            .iter()
            .filter(move |edge| &edge.to_symbol_id == symbol_id)
    }
}

/// Run the post-pass over `graph`'s own [`CodeGraph::resolved_calls`]
/// (already index-aligned with [`CodeGraph::calls`] -- see that field's
/// doc comment), producing a [`DataFlowGraph`]. A call produces zero
/// edges when it has no captured [`CallEdge::arg_texts`] (nothing to
/// link) or when [`ResolvedCall::confidence`] is
/// [`ResolutionConfidence::Unresolved`] (nothing to link *to*) -- never
/// a fabricated placeholder edge for either case. An
/// [`ResolutionConfidence::Ambiguous`] call produces one edge per
/// candidate, all sharing the same `argument_exprs`/`line`/
/// `from_symbol_id`, so a consumer sees every real possibility.
pub fn materialize(graph: &CodeGraph) -> DataFlowGraph {
    materialize_from(graph.calls(), graph.resolved_calls())
}

/// Same as [`materialize`] but takes an explicit `(calls, resolved)`
/// pair rather than pulling both from a [`CodeGraph`] -- lets a caller
/// that already has a fresh [`resolution::resolve`] result (e.g. a test,
/// or [`crate::analysis::trace`] resolving against an adjacency snapshot
/// mid-build) avoid recomputing resolution a second time.
/// `calls`/`resolved` MUST be the same length and index-aligned (the same
/// contract [`CodeGraph::resolved_calls`] documents); a caller that
/// violates this gets a graph built only over the shared prefix rather
/// than a panic, since this pass never panics on caller-supplied data.
pub fn materialize_from(calls: &[CallEdge], resolved: &[ResolvedCall]) -> DataFlowGraph {
    let mut edges = Vec::new();

    for (call, resolved_call) in calls.iter().zip(resolved.iter()) {
        if call.arg_texts.is_empty() {
            continue;
        }
        if resolved_call.confidence == ResolutionConfidence::Unresolved {
            continue;
        }
        for candidate in &resolved_call.candidates {
            edges.push(DataFlowEdge {
                from_symbol_id: resolved_call
                    .from_symbol_id
                    .as_ref()
                    .map(|id| id.as_str().into()),
                to_symbol_id: candidate.as_str().into(),
                confidence: resolved_call.confidence,
                argument_exprs: call
                    .arg_texts
                    .iter()
                    .map(|argument| argument.as_str().into())
                    .collect(),
                line: call.line,
            });
        }
    }

    DataFlowGraph { edges }
}

/// Convenience wrapper: run [`resolution::resolve`] fresh over `graph`
/// and immediately materialize -- for a caller that has not already
/// computed a resolution pass (most callers should prefer [`materialize`]
/// against an already-indexed [`CodeGraph`], whose
/// [`CodeGraph::resolved_calls`] this avoids recomputing).
pub fn materialize_fresh(graph: &CodeGraph) -> DataFlowGraph {
    let resolved = resolution::resolve(graph);
    materialize_from(graph.calls(), &resolved)
}

/// Build a lookup from every resolved target symbol id in `graph` back
/// to its [`DataFlowEdge`]s' argument expressions -- a small helper for
/// a caller (e.g. [`crate::analysis::trace`]) that wants "what argument
/// expressions has anyone ever passed into this symbol" without walking
/// [`DataFlowGraph::edges_to_symbol`] once per hop.
pub fn argument_exprs_by_target(
    data_flow: &DataFlowGraph,
) -> HashMap<MemoryDataFlowTargetSymbolId, Vec<&MemoryDataFlowArgumentExpression>> {
    let mut by_target = HashMap::new();
    for edge in data_flow.edges() {
        let exprs = by_target
            .entry(edge.to_symbol_id.retained())
            .or_insert_with(Vec::new);
        for arg in &edge.argument_exprs {
            exprs.push(arg);
        }
    }
    by_target
}
