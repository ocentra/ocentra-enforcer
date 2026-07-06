//! X06.3: graph algorithms over [`crate::code_graph::CodeGraph`].
//!
//! `code_graph` deliberately stores nodes as a flat vec plus typed edge
//! lists (import/call/route) that reference other nodes only by string
//! id -- resolution and traversal are this module's job (see
//! `code_graph`'s own module docs: "resolved lazily by callers that
//! need traversal (X06.3's concern)").
//!
//! This module builds a `petgraph::graph::DiGraph` adjacency view once
//! per query session ([`CodeAdjacency::build`]) and layers the
//! workpack's required algorithms on top of it:
//!
//! - [`CodeAdjacency::related`] -- bounded-depth related-node walk;
//! - [`CodeAdjacency::trace_calls`] -- call-path tracing (in/out/both);
//! - [`CodeAdjacency::reverse_dependents`] -- reverse dependency
//!   traversal (upstream callers/importers);
//! - [`CodeAdjacency::hotspots`] -- centrality/hotspot detection
//!   (betweenness-style degree ranking via petgraph);
//!
//! plus the submodules [`query`] (the read-only Cypher-subset DSL,
//! D-05) and [`trace`] (X06.P2: parity `trace_path` modes
//! calls/data_flow/cross_service) that sit on top of the same adjacency.
//!
//! # Why petgraph
//!
//! BORROW_POLICY + DECISIONS D-04 context name petgraph as an approved
//! dependency for exactly this class of algorithm (dijkstra,
//! centrality, community detection) rather than hand-rolling BFS/DFS by
//! hand for every traversal shape. This module is a from-scratch
//! re-expression over enforcer's own node/edge model -- no code is
//! copied from any harvested source (harvested-from: none; petgraph
//! itself is the borrowed *dependency*, not borrowed *code*, per
//! MEMORY_RETRIEVAL_BORROW_POLICY §2 TabAgentServer row: "Cargo
//! dependency CHOICES may be adopted directly").

pub mod clustering;
pub mod query;
pub mod trace;

use crate::code_graph::{CodeGraph, CodeNode};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use std::collections::{HashMap, HashSet, VecDeque};

/// The kind of relationship an [`AdjacencyEdge`] represents, carried
/// alongside the petgraph edge so traversal results can explain *why*
/// two nodes are connected (the workpack's "traversal reasoning" idea,
/// harvested-idea-only from the MIA framework digest: "found via 2-hop:
/// A -> implies -> B" translated to enforcer's own edge kinds).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeKind {
    /// `from` imports `to` (resolved import -> file target).
    Imports,
    /// `from` calls a symbol resolved to `to`.
    Calls,
    /// `from` declares an HTTP route (synthetic route node -> file).
    Route,
    /// `from` file contains symbol `to` (structural containment).
    Contains,
    /// X06 rich vocabulary (additive): `from` (a Class/Interface)
    /// inherits/extends a symbol resolved to `to`.
    Inherits,
    /// `from` (a type) implements a trait/interface resolved to `to`.
    Implements,
    /// `from` (a symbol) is decorated by a symbol/macro resolved to `to`.
    Decorates,
    /// `from` (a symbol)'s signature references a type resolved to `to`.
    TypeRef,
    /// `from` (a container symbol) defines member symbol `to`.
    Defines,
    /// X06 core parity: `from` (a calling symbol) has a resolved call
    /// site with at least one captured argument expression flowing into
    /// `to` (the resolved callee) -- [`crate::data_flow`]'s
    /// materialization of the baseline's `DATA_FLOWS` edge at the
    /// argument-expression granularity this crate's parser layer
    /// supports (see that module's doc comment for the full baseline
    /// citation). Additive to, never a replacement for, the symbol-
    /// scoped `Calls` edge [`CodeAdjacency::build`] already adds for the
    /// same resolved call -- a `DataFlows` edge only ever exists
    /// alongside a `Calls` edge between the same two nodes, so a
    /// `calls`-mode trace is unaffected by this variant's existence.
    DataFlows,
}

/// One traversal step: the edge kind plus the node id reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelatedNode {
    pub node_id: String,
    pub depth: usize,
    pub via: EdgeKind,
}

/// A single hop in a traced call/import path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathHop {
    pub node_id: String,
    pub via: EdgeKind,
}

/// One node's computed hotspot score: raw in+out degree, used as the
/// centrality proxy (see [`CodeAdjacency::hotspots`] docs for why
/// degree rather than full betweenness is the right first metric here).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotspotScore {
    pub node_id: String,
    pub in_degree: usize,
    pub out_degree: usize,
}

impl HotspotScore {
    pub fn total_degree(&self) -> usize {
        self.in_degree + self.out_degree
    }
}

/// Direction to traverse call/import edges in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceDirection {
    /// Follow edges as declared (caller -> callee, importer -> imported).
    Out,
    /// Follow edges backwards (callee -> caller, imported -> importer).
    In,
    /// Follow both directions.
    Both,
}

/// A resolved import/call adjacency view over one [`CodeGraph`]
/// snapshot. Import module paths and call callee names are resolved to
/// concrete file/symbol node ids on a best-effort basis (exact
/// qualified-path resolution across an arbitrary build system is out of
/// scope, matching `code_graph`'s own module docs); an edge whose target
/// cannot be resolved to a node already in the graph is dropped rather
/// than fabricating a dangling node.
pub struct CodeAdjacency {
    graph: DiGraph<String, EdgeKind>,
    index_of: HashMap<String, NodeIndex>,
    /// Every function/type/test symbol's containing file id, keyed by
    /// symbol id. [`CallEdge`](crate::code_graph::CallEdge)/
    /// [`ImportEdge`](crate::code_graph::ImportEdge) are recorded at
    /// file granularity only (`code_graph`'s own module docs: no
    /// enclosing-symbol data in the parser layer), so a `Calls`/`Imports`
    /// edge's source is always a *file* node, never a symbol node --
    /// see [`CodeAdjacency::trace_calls`]'s "symbol start bridging" note
    /// for why this map exists.
    symbol_file_of: HashMap<String, String>,
}

impl CodeAdjacency {
    /// Build the adjacency view from a [`CodeGraph`] snapshot.
    /// Linear in the graph's node and edge count; callers that need
    /// repeated queries against the same snapshot should build once
    /// and reuse.
    pub fn build(graph: &CodeGraph) -> Self {
        let mut g = DiGraph::<String, EdgeKind>::new();
        let mut index_of: HashMap<String, NodeIndex> = HashMap::new();

        for node in graph.nodes() {
            let id = node.id().to_string();
            index_of.entry(id.clone()).or_insert_with(|| g.add_node(id));
        }

        // File -> symbol containment edges (a symbol's file_id is
        // always a node already inserted above).
        let mut symbol_file_of: HashMap<String, String> = HashMap::new();
        for symbol in graph.symbol_nodes() {
            symbol_file_of.insert(symbol.id.clone(), symbol.file_id.clone());
            if let (Some(&from), Some(&to)) =
                (index_of.get(&symbol.file_id), index_of.get(&symbol.id))
            {
                g.add_edge(from, to, EdgeKind::Contains);
            }
        }

        // Import edges: resolve `module_path` to a file node by suffix
        // match against every known rel_path (best-effort, see struct
        // docs) -- e.g. `module_path` "fs" or "./util" matching a file
        // ending in the same stem.
        let file_ids: Vec<(&str, NodeIndex)> = graph
            .file_nodes()
            .filter_map(|f| {
                index_of
                    .get(f.id.as_str())
                    .map(|&idx| (f.rel_path.as_str(), idx))
            })
            .collect();

        for import in graph.imports() {
            let Some(&from) = index_of.get(&import.from_file_id) else {
                continue;
            };
            if let Some(&to) = resolve_module_path(&import.module_path, &file_ids) {
                if to != from {
                    g.add_edge(from, to, EdgeKind::Imports);
                }
            }
        }

        // Call edges: resolve `callee` to a symbol node by exact-name
        // match (best-effort; see struct docs).
        let symbol_ids: Vec<(&str, NodeIndex)> = graph
            .symbol_nodes()
            .filter_map(|s| {
                index_of
                    .get(s.id.as_str())
                    .map(|&idx| (s.name.as_str(), idx))
            })
            .collect();

        // X06 type-aware resolution: `graph.resolved_calls()` is
        // index-aligned with `graph.calls()` (see both types' own
        // docs) -- when a resolved entry names exactly one candidate
        // symbol (`Resolved`/`Probable`), ADD a symbol-scoped Calls edge
        // from the *calling symbol* (not just its file) to that
        // candidate, in addition to (not instead of) the original
        // file-scoped best-effort name match below. Keeping both is
        // deliberate: the file-scoped edge is what every existing
        // file-rooted trace (this struct's own pre-existing "symbol-
        // start bridging" note) already depends on for its hop-count
        // budget, and dropping it would silently add an extra
        // File--Contains-->Symbol hop to every trace that used to reach
        // the callee in one hop from the file -- a regression this
        // pass must not introduce. The symbol-scoped edge is additive:
        // it gives a symbol-rooted trace a direct, precise hop that
        // does not need the file-bridging fallback at all.
        let resolved_calls = graph.resolved_calls();
        for (i, call) in graph.calls().iter().enumerate() {
            let Some(&from_file) = index_of.get(&call.from_file_id) else {
                continue;
            };

            let resolved = resolved_calls.get(i);
            let single_resolved_candidate = resolved.and_then(|r| {
                matches!(
                    r.confidence,
                    crate::resolution::ResolutionConfidence::Resolved
                        | crate::resolution::ResolutionConfidence::Probable
                )
                .then(|| r.candidates.first())
                .flatten()
            });

            if let Some(target_id) = single_resolved_candidate {
                if let Some(&to) = index_of.get(target_id.as_str()) {
                    if let Some(from_symbol) = resolved
                        .and_then(|r| r.from_symbol_id.as_deref())
                        .and_then(|id| index_of.get(id))
                        .copied()
                    {
                        g.add_edge(from_symbol, to, EdgeKind::Calls);
                    }
                }
            }

            if let Some(&to) = resolve_callee(&call.callee, &symbol_ids) {
                g.add_edge(from_file, to, EdgeKind::Calls);
            } else if let Some(target_id) = single_resolved_candidate {
                // The old name-based `resolve_callee` missed this call
                // (e.g. the callee text is `self.report` and no symbol
                // is literally named `self.report`/`report` uniquely by
                // that heuristic's own rule) but type-aware resolution
                // still found a target -- keep the file-scoped edge
                // available too, so a file-rooted trace still reaches
                // it in one hop same as it would for any other resolved
                // callee.
                if let Some(&to) = index_of.get(target_id.as_str()) {
                    g.add_edge(from_file, to, EdgeKind::Calls);
                }
            }
        }

        // Route edges: synthetic edge from the declaring file to itself
        // is meaningless, so routes are modeled as a self-loop-free
        // marker: skipped here (route metadata lives on `CodeGraph`
        // directly; `architecture::route_map` reads it from there).
        let _ = graph.routes();

        // X06 core parity: `DataFlows` edges from `crate::data_flow`'s
        // post-pass -- one symbol-scoped edge per (resolved call, has
        // captured arguments) pair, added alongside (never instead of)
        // the `Calls` edge the loop above already adds for the same
        // resolved call. A `DataFlowEdge` with no `from_symbol_id` (a
        // module-scope call site) is skipped here -- `EdgeKind::DataFlows`
        // is defined only between two symbol nodes in this adjacency, the
        // same restriction the `Calls` symbol-scoped edge above already
        // has; the file-scoped `Calls` edge already covers that case for
        // traversal purposes.
        for edge in crate::data_flow::materialize(graph).edges() {
            let Some(from_symbol_id) = edge.from_symbol_id.as_deref() else {
                continue;
            };
            if let (Some(&from), Some(&to)) = (
                index_of.get(from_symbol_id),
                index_of.get(edge.to_symbol_id.as_str()),
            ) {
                g.add_edge(from, to, EdgeKind::DataFlows);
            }
        }

        // X06 rich vocabulary (additive): INHERITS/IMPLEMENTS/DECORATES/
        // TYPE_REF resolve their unresolved-by-name target against the
        // same `symbol_ids` name index calls/imports already build
        // (best-effort, matching every other edge kind's own resolution
        // rationale). DEFINES already carries both ends as resolved ids
        // (both extracted from the same file, same pass), so it needs no
        // name lookup at all.
        for edge in graph.inherits() {
            let Some(&from) = index_of.get(edge.sub_id.as_str()) else {
                continue;
            };
            if let Some(&to) = resolve_callee(&edge.super_name, &symbol_ids) {
                g.add_edge(from, to, EdgeKind::Inherits);
            }
        }
        for edge in graph.implements() {
            let Some(&from) = index_of.get(edge.type_id.as_str()) else {
                continue;
            };
            if let Some(&to) = resolve_callee(&edge.trait_name, &symbol_ids) {
                g.add_edge(from, to, EdgeKind::Implements);
            }
        }
        for edge in graph.decorates() {
            let Some(&from) = index_of.get(edge.target_id.as_str()) else {
                continue;
            };
            if let Some(&to) = resolve_callee(&edge.decorator_name, &symbol_ids) {
                g.add_edge(from, to, EdgeKind::Decorates);
            }
        }
        for edge in graph.type_refs() {
            let Some(&from) = index_of.get(edge.from_id.as_str()) else {
                continue;
            };
            if let Some(&to) = resolve_callee(&edge.type_name, &symbol_ids) {
                g.add_edge(from, to, EdgeKind::TypeRef);
            }
        }
        for edge in graph.defines() {
            if let (Some(&from), Some(&to)) = (
                index_of.get(edge.container_id.as_str()),
                index_of.get(edge.member_id.as_str()),
            ) {
                g.add_edge(from, to, EdgeKind::Defines);
            }
        }

        Self {
            graph: g,
            index_of,
            symbol_file_of,
        }
    }

    pub fn contains_node(&self, node_id: &str) -> bool {
        self.index_of.contains_key(node_id)
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Bounded-depth related-node walk (BFS) from `start`, following
    /// edges in both directions. `max_depth` is a hard ceiling -- the
    /// workpack's "graph depth limit" hard test asserts nodes beyond
    /// this depth are never returned, even on a graph that connects
    /// further.
    pub fn related(&self, start: &str, max_depth: usize) -> Vec<RelatedNode> {
        let Some(&start_idx) = self.index_of.get(start) else {
            return Vec::new();
        };
        let mut state = RelatedWalkState {
            visited: HashSet::new(),
            frontier: VecDeque::new(),
            out: Vec::new(),
        };
        state.visited.insert(start_idx);
        state.frontier.push_back((start_idx, 0));

        while let Some((idx, depth)) = state.frontier.pop_front() {
            if depth >= max_depth {
                continue;
            }
            for edge in self.graph.edges_directed(idx, Direction::Outgoing) {
                push_related(&self.graph, &mut state, edge, depth);
            }
            for edge in self.graph.edges_directed(idx, Direction::Incoming) {
                push_related(&self.graph, &mut state, edge, depth);
            }
        }
        state.out
    }

    /// Trace a call/import path from `start` up to `max_depth` hops in
    /// `direction`. Returns every distinct path as a list of hops (the
    /// first hop is the first edge taken away from `start`); `start`
    /// itself is not included as a hop.
    pub fn trace_calls(
        &self,
        start: &str,
        direction: TraceDirection,
        max_depth: usize,
    ) -> Vec<Vec<PathHop>> {
        let Some(&start_idx) = self.index_of.get(start) else {
            return Vec::new();
        };
        // Symbol-start bridging: `Calls`/`Imports` edges are recorded at
        // file granularity only (`CallEdge`/`ImportEdge` carry
        // `from_file_id`, never an enclosing-symbol id -- see
        // `symbol_file_of`'s docs), so a symbol node's own outgoing
        // edges in `self.graph` are just its `Contains` edge FROM its
        // file, never a `Calls`/`Imports` edge itself. Tracing `Out`
        // from a symbol id (as every baseline-parity caller does --
        // `trace_path`'s root is always a resolved function/symbol, per
        // the baseline spec) would therefore always dead-end at hop 0
        // without this bridge: also seed the walk from the symbol's
        // containing file node so its file-level Calls/Imports edges are
        // reachable, while still reporting/keying the walk under the
        // original symbol `start` id (the file node is never itself
        // emitted as a spurious extra hop -- see `dfs_paths`' `extra_roots`
        // handling).
        let file_idx = self
            .symbol_file_of
            .get(start)
            .and_then(|file_id| self.index_of.get(file_id))
            .copied();

        let mut state = DfsPathState {
            current: Vec::new(),
            paths: Vec::new(),
            on_path: HashSet::new(),
        };
        state.on_path.insert(start_idx);
        if let Some(file_idx) = file_idx {
            dfs_paths(&self.graph, file_idx, direction, max_depth, &mut state);
        }
        state.on_path.remove(&start_idx);
        dfs_paths(&self.graph, start_idx, direction, max_depth, &mut state);
        state.paths
    }

    /// Reverse dependency traversal: every node that (transitively, up
    /// to `max_depth`) imports or calls into `target`. This is
    /// [`Self::trace_calls`] with [`TraceDirection::In`], flattened to
    /// the unique set of upstream node ids (the workpack's "reverse
    /// dependency traversal" / "upstream callers" hard requirement).
    pub fn reverse_dependents(&self, target: &str, max_depth: usize) -> Vec<String> {
        let mut seen = HashSet::new();
        for path in self.trace_calls(target, TraceDirection::In, max_depth) {
            for hop in path {
                seen.insert(hop.node_id);
            }
        }
        let mut out: Vec<String> = seen.into_iter().collect();
        out.sort();
        out
    }

    /// Centrality/hotspot detection: rank every node by total (in +
    /// out) degree, descending. Degree centrality is the workpack's
    /// minimum-viable hotspot metric (a file called/imported from many
    /// places, or that calls/imports many others, is structurally
    /// significant) and is exact and O(V+E) -- no sampling, no
    /// approximation -- unlike betweenness/eigenvector variants which
    /// this module does not (yet) need for the hard tests.
    pub fn hotspots(&self, limit: usize) -> Vec<HotspotScore> {
        let mut scores: Vec<HotspotScore> = self
            .graph
            .node_indices()
            .map(|idx| HotspotScore {
                node_id: self.graph[idx].clone(),
                in_degree: self.graph.edges_directed(idx, Direction::Incoming).count(),
                out_degree: self.graph.edges_directed(idx, Direction::Outgoing).count(),
            })
            .collect();
        scores.sort_by(|a, b| {
            b.total_degree()
                .cmp(&a.total_degree())
                .then_with(|| a.node_id.cmp(&b.node_id))
        });
        scores.truncate(limit);
        scores
    }

    fn node_ids(&self) -> impl Iterator<Item = &str> {
        self.graph
            .node_indices()
            .map(move |idx| self.graph[idx].as_str())
    }
}

/// Mutable accumulator threaded through [`dfs_paths`]: the in-progress
/// hop list, every completed path found so far, and the set of nodes
/// on the current DFS stack (cycle guard). Bundled into one struct so
/// the free function below stays under clippy's default
/// too-many-arguments threshold without an `#[allow]`.
struct DfsPathState {
    current: Vec<PathHop>,
    paths: Vec<Vec<PathHop>>,
    on_path: HashSet<NodeIndex>,
}

/// Depth-bounded DFS path enumeration, factored out of
/// [`CodeAdjacency::trace_calls`] as a free function (rather than an
/// `&self` method) so it takes the graph by explicit reference on every
/// recursive call -- avoiding the `only_used_in_recursion` clippy lint
/// an `&self`-recursing method would otherwise trip (this crate runs
/// clippy with zero `#[allow(clippy::…)]`, per the workpack gate).
fn dfs_paths(
    graph: &DiGraph<String, EdgeKind>,
    idx: NodeIndex,
    direction: TraceDirection,
    remaining: usize,
    state: &mut DfsPathState,
) {
    if remaining == 0 {
        if !state.current.is_empty() {
            state.paths.push(state.current.clone());
        }
        return;
    }
    state.on_path.insert(idx);
    let mut extended = false;

    let directions: &[Direction] = match direction {
        TraceDirection::Out => &[Direction::Outgoing],
        TraceDirection::In => &[Direction::Incoming],
        TraceDirection::Both => &[Direction::Outgoing, Direction::Incoming],
    };

    for &dir in directions {
        for edge in graph.edges_directed(idx, dir) {
            let target = other_end(dir, &edge);
            if state.on_path.contains(&target) {
                continue;
            }
            extended = true;
            state.current.push(PathHop {
                node_id: graph[target].clone(),
                via: *edge.weight(),
            });
            dfs_paths(graph, target, direction, remaining - 1, state);
            state.current.pop();
        }
    }

    if !extended && !state.current.is_empty() {
        state.paths.push(state.current.clone());
    }
    state.on_path.remove(&idx);
}

fn other_end(dir: Direction, edge: &petgraph::graph::EdgeReference<'_, EdgeKind>) -> NodeIndex {
    match dir {
        Direction::Outgoing => edge.target(),
        Direction::Incoming => edge.source(),
    }
}

/// Mutable accumulator threaded through [`push_related`]: the visited
/// set, the BFS frontier, and the accumulated results. Bundled into
/// one struct so [`CodeAdjacency::related`]'s per-edge helper stays
/// under clippy's default too-many-arguments threshold without an
/// `#[allow]`.
struct RelatedWalkState {
    visited: HashSet<NodeIndex>,
    frontier: VecDeque<(NodeIndex, usize)>,
    out: Vec<RelatedNode>,
}

fn push_related(
    graph: &DiGraph<String, EdgeKind>,
    state: &mut RelatedWalkState,
    edge: petgraph::graph::EdgeReference<'_, EdgeKind>,
    depth: usize,
) {
    // `edges_directed(idx, Outgoing)` gives edges where `idx` is the
    // source; `edges_directed(idx, Incoming)` gives edges where `idx`
    // is the target. Either way the "other" node is whichever endpoint
    // is not the current frontier node -- petgraph always reports
    // `source()`/`target()` per the edge's own direction, so pick the
    // one that is not already visited/self.
    let a = edge.source();
    let b = edge.target();
    let other = if state.visited.contains(&a) && a != b {
        b
    } else {
        a
    };
    let other = if state.visited.contains(&other) {
        if other == a {
            b
        } else {
            a
        }
    } else {
        other
    };
    if state.visited.insert(other) {
        state.out.push(RelatedNode {
            node_id: graph[other].clone(),
            depth: depth + 1,
            via: *edge.weight(),
        });
        state.frontier.push_back((other, depth + 1));
    }
}

/// Best-effort import-path resolution: a `module_path` resolves to a
/// file whose `rel_path` ends with the module path (normalized) or
/// whose file stem equals the module path's final segment. Returns the
/// first match in `file_ids` order (deterministic: `file_ids` is built
/// from `CodeGraph::file_nodes()`, itself insertion-ordered).
fn resolve_module_path<'a>(
    module_path: &str,
    file_ids: &'a [(&'a str, NodeIndex)],
) -> Option<&'a NodeIndex> {
    let needle = module_path
        .trim_start_matches("./")
        .trim_start_matches("../");
    let last_segment = needle.rsplit(['/', ':', '.']).next().unwrap_or(needle);
    if last_segment.is_empty() {
        return None;
    }
    file_ids
        .iter()
        .find(|(rel_path, _)| {
            let stem = rel_path.rsplit('/').next().unwrap_or(rel_path);
            let stem = stem.split('.').next().unwrap_or(stem);
            stem == last_segment || rel_path.ends_with(last_segment)
        })
        .map(|(_, idx)| idx)
}

/// Best-effort call resolution: `callee` resolves to a symbol node with
/// an exactly-matching name, or whose name is the callee's final
/// path/method segment (`module::func` or `obj.method` -> `func`/`method`).
fn resolve_callee<'a>(
    callee: &str,
    symbol_ids: &'a [(&'a str, NodeIndex)],
) -> Option<&'a NodeIndex> {
    let last_segment = callee.rsplit(['.', ':']).next().unwrap_or(callee);
    symbol_ids
        .iter()
        .find(|(name, _)| *name == callee || *name == last_segment)
        .map(|(_, idx)| idx)
}

/// Every node id in `graph` that is a [`CodeNode::Test`] symbol, PLUS
/// the `file:`-id of every file that contains one. The file-id half
/// matters because [`crate::code_graph::CallEdge`] (and therefore every
/// `Calls`-kind [`PathHop`]/reverse-dependent id) records only the
/// *file* a call was written in, never the enclosing symbol (see
/// `code_graph`'s own module docs) -- so a test function calling
/// something produces a hop/dependent whose id is `file:<test file>`,
/// not the test symbol's own `sym:` id. Without including the file id
/// here, `include_tests=false` (in [`trace`]) and the test-coverage
/// signal (in [`crate::impact`]) would both silently fail to exclude a
/// call that only reaches a test through its containing file's Calls
/// edge -- the exact gap this function closes for both callers.
pub(crate) fn test_node_ids(graph: &CodeGraph) -> HashSet<String> {
    let mut ids = HashSet::new();
    for node in graph.nodes() {
        if let CodeNode::Test(sym) = node {
            ids.insert(sym.id.clone());
            ids.insert(sym.file_id.clone());
        }
    }
    ids
}

/// A minimal read-only iterator surface [`query`] needs to evaluate
/// MATCH patterns without depending on petgraph's internals directly.
pub(crate) struct AdjacencyView<'a> {
    pub(crate) adjacency: &'a CodeAdjacency,
    pub(crate) graph: &'a CodeGraph,
}

impl<'a> AdjacencyView<'a> {
    pub(crate) fn new(adjacency: &'a CodeAdjacency, graph: &'a CodeGraph) -> Self {
        Self { adjacency, graph }
    }

    pub(crate) fn all_node_ids(&self) -> Vec<&str> {
        self.adjacency.node_ids().collect()
    }

    pub(crate) fn code_node(&self, id: &str) -> Option<&CodeNode> {
        self.graph.nodes().iter().find(|n| n.id() == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_graph::Manifest;
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
            adjacency.contains_node(file_a),
            "expected a.rs file node in adjacency"
        );

        let related = adjacency.related(file_a, 3);
        let ids: HashSet<&str> = related.iter().map(|r| r.node_id.as_str()).collect();
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

        // helper's symbol id is sym:b.rs:1:helper -- find it.
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
        assert!(!scores.is_empty());
        for i in 1..scores.len() {
            assert!(scores[i - 1].total_degree() >= scores[i].total_degree());
        }
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
}
